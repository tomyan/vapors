//! Screenshots by browser context, against the real browser: the
//! viewport, the virtual-time budget, isolation between screenshots, the
//! deadline, and what is left in the profile root. Each takes 1 to 4 s.

use std::path::{Path, PathBuf};
use std::time::Duration;

use super::*;
use crate::comparison::chromium_pids::{holding, kill_holding};
use crate::comparison::chromium_process::DEADLINE;

/// A scratch directory: pages, screenshots, and `profiles/`, the profile
/// root, so nothing lands in the automation cache.
pub(crate) fn scratch(name: &str) -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix(&format!("vapors-browser-{name}-"))
        .tempdir()
        .expect("a scratch directory under the temp dir")
}

/// The real browser, its profiles under `dir/profiles`.
pub(crate) fn chromium(dir: &Path, deadline: Duration) -> Chromium {
    let found = Chromium::find().unwrap_or_else(|e| panic!("{e}"));
    Chromium {
        deadline,
        profiles: dir.join("profiles"),
        ..found
    }
}

pub(crate) fn page(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(format!("{name}.html"));
    let html =
        format!("<!doctype html><style>html, body {{ margin: 0; height: 100%; }}</style>{body}");
    std::fs::write(&path, html).expect("page");
    path
}

pub(super) fn pixel(image: &Rgba, x: u32, y: u32) -> [u8; 4] {
    let at = ((y * image.width + x) * 4) as usize;
    [
        image.data[at],
        image.data[at + 1],
        image.data[at + 2],
        image.data[at + 3],
    ]
}

/// The profile root's entries, by name.
pub(crate) fn entries(root: &Path) -> Vec<String> {
    let Ok(read) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    let mut names: Vec<String> = read
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

const GREEN: [u8; 4] = [10, 200, 30, 255];

#[test]
fn screenshots_fill_the_window_at_each_scale_with_one_launch_per_scale() {
    // Given: a page filled to its last row
    let dir = scratch("viewport");
    let full = page(
        dir.path(),
        "full",
        "<div style=\"height: 100%; background: rgb(10, 200, 30)\"></div>",
    );
    let mut browser = chromium(dir.path(), DEADLINE).browser();

    // When
    let shot = |browser: &mut Browser, scale, name: &str| {
        browser
            .screenshot(&full, &dir.path().join(name), 640, 400, scale)
            .unwrap_or_else(|e| panic!("{e}"))
    };
    let (first, second, double) = (
        shot(&mut browser, 1, "a.png"),
        shot(&mut browser, 1, "b.png"),
        shot(&mut browser, 2, "c.png"),
    );

    // Then: the bottom row is the page's, not outside a smaller viewport
    assert_eq!(browser.launches(), 2);
    assert_eq!(
        (first.width, first.height, double.width, double.height),
        (640, 400, 1280, 800)
    );
    for (image, bottom) in [(&first, 399), (&second, 399), (&double, 799)] {
        assert_eq!(
            (pixel(image, 0, 0), pixel(image, 0, bottom)),
            (GREEN, GREEN)
        );
    }
}

#[test]
fn the_virtual_time_budget_runs_timers_due_within_5000_ms_and_no_later_ones() {
    // Given: one colour change due at 3 s, another at 6 s
    let dir = scratch("budget");
    let timed = page(
        dir.path(),
        "timed",
        "<div id=a style=\"height: 50%; background: red\"></div><div id=b style=\"height: 50%; background: rgb(10, 200, 30)\"></div>\
         <script>setTimeout(() => a.style.background = 'rgb(10, 200, 30)', 3000);\
         setTimeout(() => b.style.background = 'red', 6000);</script>",
    );

    // When
    let image = chromium(dir.path(), DEADLINE)
        .screenshot(&timed, &dir.path().join("timed.png"), 640, 400, 1)
        .unwrap_or_else(|e| panic!("{e}"));

    // Then
    assert_eq!((pixel(&image, 0, 0), pixel(&image, 0, 399)), (GREEN, GREEN));
}

#[test]
fn a_screenshot_sees_nothing_an_earlier_one_stored() {
    // Given: a page green until it finds its own localStorage mark
    let dir = scratch("isolation");
    let marking = page(
        dir.path(),
        "marking",
        "<div id=d style=\"height: 100%; background: rgb(10, 200, 30)\"></div>\
         <script>if (localStorage.getItem('seen')) d.style.background = 'red'; localStorage.setItem('seen', '1');</script>",
    );
    let mut browser = chromium(dir.path(), DEADLINE).browser();

    // When
    let first = browser
        .screenshot(&marking, &dir.path().join("a.png"), 640, 400, 1)
        .unwrap_or_else(|e| panic!("{e}"));
    let second = browser
        .screenshot(&marking, &dir.path().join("b.png"), 640, 400, 1)
        .unwrap_or_else(|e| panic!("{e}"));

    // Then
    assert_eq!(browser.launches(), 1);
    assert_eq!((pixel(&first, 0, 0), pixel(&second, 0, 0)), (GREEN, GREEN));
}

#[test]
fn the_profile_root_holds_only_the_browser_s_expiry_directory_and_nothing_once_closed() {
    // Given
    let dir = scratch("root");
    let full = page(dir.path(), "full", "<p>text</p>");
    let root = dir.path().join("profiles");
    let mut browser = chromium(dir.path(), DEADLINE).browser();

    // When
    browser
        .screenshot(&full, &dir.path().join("a.png"), 640, 400, 1)
        .unwrap_or_else(|e| panic!("{e}"));
    let running = entries(&root);
    let was_dir = running.first().is_some_and(|name| root.join(name).is_dir());
    let closed = browser.close();

    // Then
    assert_eq!(running.len(), 1, "{running:?}");
    assert!(running[0].starts_with("expires-") && was_dir, "{running:?}");
    assert_eq!(closed, Ok(()));
    assert_eq!(entries(&root), Vec::<String>::new());
}

#[test]
fn a_screenshot_past_its_deadline_stops_only_its_browser_and_the_next_starts_another() {
    // Given: a browser running, then a page that never yields
    let dir = scratch("deadline");
    let (fine, hang) = (
        page(dir.path(), "fine", "<p>fine</p>"),
        page(dir.path(), "hang", "<script>for (;;) {}</script>"),
    );
    let root = dir.path().join("profiles");
    let mut browser = chromium(dir.path(), DEADLINE).browser();
    browser
        .screenshot(&fine, &dir.path().join("a.png"), 640, 400, 1)
        .unwrap_or_else(|e| panic!("{e}"));
    let profile = root.join(&entries(&root)[0]);

    // When: only the hanging screenshot has the short deadline; with 3 s
    // for all three, the first timed out under the unit tier's load
    browser.chromium.deadline = Duration::from_secs(3);
    let late = browser.screenshot(&hang, &dir.path().join("b.png"), 640, 400, 1);
    browser.chromium.deadline = DEADLINE;
    let after = browser.screenshot(&fine, &dir.path().join("c.png"), 640, 400, 1);

    // Then: this process (the test's group) is alive to check
    let message = late.expect_err("past its deadline").to_string();
    assert!(
        message.contains("the screenshot, with 3s to finish"),
        "{message}"
    );
    assert_eq!(holding(&profile), Ok(Vec::new()));
    assert!(
        !profile.exists(),
        "left in the profile: {:?}",
        std::process::Command::new("find")
            .arg(&profile)
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
    );
    assert!(after.is_ok(), "{after:?}");
    assert_eq!(browser.launches(), 2);
}

#[test]
fn every_process_of_the_browser_is_found_and_killed_by_its_profile() {
    // Given: a browser up, with its zygotes, GPU, utility and renderer
    // processes, whose titles Chromium rewrites
    let dir = scratch("every");
    let fine = page(dir.path(), "fine", "<p>fine</p>");
    let root = dir.path().join("profiles");
    let mut browser = chromium(dir.path(), DEADLINE).browser();
    browser
        .screenshot(&fine, &dir.path().join("a.png"), 640, 400, 1)
        .unwrap_or_else(|e| panic!("{e}"));
    let profile = root.join(&entries(&root)[0]);

    // When: killed by profile while it runs, its main process included
    let running = holding(&profile).expect("/proc is readable");
    let killed = kill_holding(&profile);
    let left = holding(&profile);
    let closed = browser.close();

    // Then: 9 were found on 2026-11-23 (the browser and 8 it started)
    assert!(
        running.len() > 1,
        "only {running:?} found holding {}",
        profile.display()
    );
    assert_eq!(killed, Ok(()));
    assert_eq!(left, Ok(Vec::new()));
    assert_eq!(closed, Ok(()));
    assert!(!profile.exists());
}

#[test]
fn a_page_that_cannot_be_loaded_is_refused_naming_it_and_chromium_s_error() {
    // Given: a page that does not exist, which Chromium would show its
    // own error page for
    let dir = scratch("missing");
    let missing = dir.path().join("missing.html");

    // When
    let refused =
        chromium(dir.path(), DEADLINE).screenshot(&missing, &dir.path().join("a.png"), 640, 400, 1);

    // Then
    let message = refused.map(drop).expect_err("no page to load").to_string();
    assert!(
        message.contains(&missing.display().to_string()),
        "{message}"
    );
    assert!(message.contains("net::ERR_FILE_NOT_FOUND"), "{message}");
}
