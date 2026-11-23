//! A live Chromium's profile and early exit, with a stand-in binary.

use std::time::{Duration, Instant};

use super::*;

fn dir(test: &str) -> tempfile::TempDir {
    let made = tempfile::Builder::new()
        .prefix(&format!("chromium-live-{test}-"))
        .tempdir();
    made.expect("a scratch directory under the temp dir")
}

/// A "browser" that says why on stderr and exits 3.
fn failing(dir: &Path) -> Chromium {
    let script = dir.join("chromium");
    std::fs::write(&script, "#!/bin/sh\necho 'bad flag --nope' >&2\nexit 3\n").expect("script");
    let made = Command::new("chmod")
        .arg("755")
        .arg(&script)
        .status()
        .expect("chmod");
    assert!(made.success());
    Chromium {
        binary: script,
        deadline: Duration::from_secs(5),
        profiles: dir.join("profiles"),
    }
}

#[test]
fn a_chromium_that_exits_says_why_with_its_stderr() {
    // Given
    let scratch = dir("exits");
    let dir = scratch.path();
    let mut live = failing(dir)
        .live(&dir.join("profile"), 400, 300, 1)
        .expect("starts");

    // When
    let started = Instant::now();
    let why = loop {
        if let Err(why) = live.alive() {
            break why;
        }
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "it never exited"
        );
        std::thread::sleep(Duration::from_millis(20));
    };

    // Then
    assert!(why.contains("exit status: 3"), "{why}");
    assert!(why.contains("bad flag --nope"), "{why}");
}

#[test]
fn a_profile_with_contents_is_refused() {
    // Given
    let scratch = dir("used");
    let dir = scratch.path();
    let profile = dir.join("profile");
    std::fs::create_dir_all(&profile).expect("profile");
    std::fs::write(
        profile.join("DevToolsActivePort"),
        "9222\n/devtools/browser/x",
    )
    .expect("stale");

    // When
    let refusal = failing(dir)
        .live(&profile, 400, 300, 1)
        .map(|_| ())
        .expect_err("refused");

    // Then
    assert!(refusal.to_string().contains("is not empty"), "{refusal}");
}

#[test]
fn a_live_browser_writes_nothing_beside_its_profile_and_leaves_nothing_once_dropped() {
    // Given: its profile in a root of its own, as in the automation cache,
    // where the retention job removes profile directories and nothing else
    let scratch = dir("beside");
    let root = scratch.path().join("profiles");
    let profile = root.join("expires-2000-01-01-x");
    let names = || -> Vec<String> {
        std::fs::read_dir(&root).map_or_else(
            |_| Vec::new(),
            |read| {
                read.map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
                    .collect()
            },
        )
    };

    // When
    let live = failing(scratch.path())
        .live(&profile, 400, 300, 1)
        .expect("starts");
    let running = names();
    drop(live);

    // Then
    assert_eq!(running, vec!["expires-2000-01-01-x".to_owned()]);
    assert_eq!(names(), Vec::<String>::new());
}
