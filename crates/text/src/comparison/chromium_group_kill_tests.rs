//! A test process killed with SIGKILL while each launcher has Chromium
//! running: as nextest kills a timed-out test, its whole process group.
//! `Drop` never runs, so only Chromium being in that group stops it. Each
//! test reruns this binary as the holder (a `helper_` test below) and
//! kills the holder's group once a process holds the profile.
//!
//! The holders: a [`Browser`](crate::comparison::Browser) after a
//! screenshot, a `dump_dom` of a page that never yields, and a
//! [`Live`](crate::comparison::Live) browser. The reused browser's
//! processes were gone 26 to 61 ms after the kill (3 runs, 2026-11-23);
//! the wait for them is [`KILLED_END_WITHIN`], 5 s.

use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use crate::comparison::chromium_browser::tests::{chromium, entries, page, scratch};
use crate::comparison::chromium_pids::{KILLED_END_WITHIN, holding, kill_holding};
use crate::comparison::chromium_process::DEADLINE;

const DIR_ENV: &str = "VAPORS_CHROMIUM_HOLDER_DIR";

/// How long the holder may take to have Chromium up: its launch has the
/// launcher's deadline, and 5 s more is for the holder process itself.
/// From spawning the holder to its browser's screenshot took 0.49 to
/// 1.05 s in 8 runs at load 13 (2026-11-23), so 5 s is about five times
/// all of it.
const UP_WITHIN: Duration = Duration::from_secs(DEADLINE.as_secs() + 5);

/// The holder's directory when this process is a holder.
fn holder_dir() -> Option<PathBuf> {
    std::env::var_os(DIR_ENV).map(PathBuf::from)
}

#[test]
#[ignore = "a holder process of the SIGKILL tests; it runs only from there"]
fn helper_holds_a_browser() {
    let Some(dir) = holder_dir() else { return };
    let fine = page(&dir, "fine", "<p>held</p>");
    let mut browser = chromium(&dir, DEADLINE).browser();
    browser
        .screenshot(&fine, &dir.join("held.png"), 640, 400, 1)
        .unwrap_or_else(|e| panic!("{e}"));
    std::thread::sleep(UP_WITHIN);
}

#[test]
#[ignore = "a holder process of the SIGKILL tests; it runs only from there"]
fn helper_holds_a_dom_dump() {
    let Some(dir) = holder_dir() else { return };
    let hang = page(&dir, "hang", "<script>for (;;) {}</script>");
    let _ = chromium(&dir, UP_WITHIN).dump_dom(&hang, 640, 400, 1.0);
}

#[test]
#[ignore = "a holder process of the SIGKILL tests; it runs only from there"]
fn helper_holds_a_live_browser() {
    let Some(dir) = holder_dir() else { return };
    let profile = dir.join("profiles").join("expires-2000-01-01-live");
    let _live = chromium(&dir, DEADLINE)
        .live(&profile, 400, 300, 1)
        .unwrap_or_else(|e| panic!("{e}"));
    std::thread::sleep(UP_WITHIN);
}

/// `helper` started in a process group of its own, as nextest runs a
/// test, with `dir` for its pages and profiles and `tmp` as its TMPDIR.
fn start_holder(helper: &str, dir: &Path, tmp: &Path) -> Child {
    let exe = std::env::current_exe().expect("this test binary");
    let name = format!("comparison::chromium_pids::group_kill_tests::{helper}");
    Command::new(exe)
        .args([name.as_str(), "--exact", "--ignored", "--nocapture"])
        .env(DIR_ENV, dir)
        .env("TMPDIR", tmp)
        .process_group(0)
        .stdout(Stdio::null())
        .spawn()
        .expect("the holder starts")
}

/// The profile under `root` once a process holds it, by [`UP_WITHIN`].
fn held_profile(root: &Path) -> Option<PathBuf> {
    let started = Instant::now();
    while started.elapsed() < UP_WITHIN {
        let profile = entries(root).first().map(|name| root.join(name));
        if let Some(profile) = profile.filter(|p| holding(p).is_ok_and(|pids| !pids.is_empty())) {
            return Some(profile);
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    None
}

/// SIGKILL to `holder`'s process group; then up to [`KILLED_END_WITHIN`]
/// for `profile`'s processes to end. Whether the kill ran, and the
/// processes left (killed after, so none outlives the test).
fn kill_group(holder: &mut Child, profile: &Path) -> (bool, Vec<u32>) {
    let group = format!("-{}", holder.id());
    let killed = Command::new("kill")
        .args(["-s", "KILL", "--", &group])
        .status()
        .is_ok_and(|s| s.success());
    let _ = holder.wait();
    let started = Instant::now();
    while !holding(profile).expect("/proc is readable").is_empty()
        && started.elapsed() < KILLED_END_WITHIN
    {
        std::thread::sleep(Duration::from_millis(20));
    }
    let left = holding(profile).expect("/proc is readable");
    let _ = kill_holding(profile);
    (killed, left)
}

/// The holder `helper` killed with its group once Chromium is up: no
/// process holding its profile is left, and its launch's TMPDIR is left
/// in the holder's, which goes when this returns. That TMPDIR is
/// directly under the temp dir, as short as `launch_tmp`'s: Chromium's
/// socket path in it must fit in 108 bytes, which one nested deeper did
/// not under xtask's TMPDIR.
fn assert_killed_with_its_group(helper: &str) {
    // Given
    let dir = scratch("killed");
    let held = tempfile::Builder::new()
        .prefix("vk-")
        .tempdir()
        .expect("the holder's TMPDIR");
    let mut holder = start_holder(helper, dir.path(), held.path());
    let profile = held_profile(&dir.path().join("profiles"));

    // When
    let (killed, left) = profile
        .as_ref()
        .map_or((false, Vec::new()), |p| kill_group(&mut holder, p));

    // Then
    let _ = (holder.kill(), holder.wait());
    assert!(
        profile.is_some(),
        "{helper}: no process held a profile within {UP_WITHIN:?}"
    );
    assert!(killed, "{helper}: kill -s KILL -- -<holder>");
    assert_eq!(
        left,
        Vec::<u32>::new(),
        "{helper}: Chromium outlived its test"
    );
    let tmps = entries(held.path());
    assert!(
        tmps.len() == 1 && tmps[0].starts_with("chromium-"),
        "{helper}'s TMPDIR: {tmps:?}"
    );
}

#[test]
fn a_test_killed_with_its_group_leaves_no_process_of_its_reused_browser() {
    assert_killed_with_its_group("helper_holds_a_browser");
}

#[test]
fn a_test_killed_with_its_group_leaves_no_process_of_its_dom_dump() {
    assert_killed_with_its_group("helper_holds_a_dom_dump");
}

#[test]
fn a_test_killed_with_its_group_leaves_no_process_of_its_live_browser() {
    assert_killed_with_its_group("helper_holds_a_live_browser");
}
