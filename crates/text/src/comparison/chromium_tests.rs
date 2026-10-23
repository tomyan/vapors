//! The runner's checks on plain data: stale files and sizes.

use super::*;

#[test]
fn a_stale_screenshot_is_removed_and_a_missing_one_is_fine() {
    // Given
    let dir = std::env::temp_dir().join(format!("vapors-chromium-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("tmp dir");
    let png = dir.join("stale.png");
    std::fs::write(&png, b"old").expect("write");

    // When
    remove_stale(&png).expect("removed");
    let again = remove_stale(&png);

    // Then
    assert!(!png.exists());
    assert_eq!(again, Ok(()));
    std::fs::remove_dir_all(&dir).expect("cleanup");
}

#[test]
fn a_screenshot_of_the_wrong_size_is_refused_naming_both_sizes() {
    // Given
    let image = Rgba {
        width: 10,
        height: 5,
        data: vec![0; 200],
    };

    // When
    let error = check_size(&image, Path::new("x.png"), [20, 10]).expect_err("wrong size");

    // Then
    let message = error.to_string();
    assert!(message.contains("10x5, wanted 20x10"), "{message}");
    assert!(message.contains("Fix:"), "{message}");
}

#[test]
fn a_browser_past_its_deadline_is_killed_with_its_children_and_its_stderr_reported() {
    // Given: a "browser" that writes to stderr, leaves a child holding
    // the pipe, then hangs
    let dir = std::env::temp_dir().join(format!("vapors-chromium-hang-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("tmp dir");
    let script = dir.join("hang.sh");
    let child_pid = dir.join("child.pid");
    let text = format!(
        "#!/bin/sh\necho 'gpu process died' >&2\nsleep 300 &\necho $! > {}\nsleep 300\n",
        child_pid.display()
    );
    std::fs::write(&script, text).expect("script");
    let made = std::process::Command::new("chmod")
        .args(["755"])
        .arg(&script)
        .status()
        .expect("chmod");
    assert!(made.success());
    let chromium = Chromium {
        binary: script,
        deadline: std::time::Duration::from_secs(1),
    };

    // When
    let started = std::time::Instant::now();
    let error = chromium
        .screenshot(&dir.join("page.html"), &dir.join("page.png"), 10, 10, 1)
        .expect_err("hung");

    // Then
    assert!(
        started.elapsed() < std::time::Duration::from_secs(10),
        "{:?}",
        started.elapsed()
    );
    let message = error.to_string();
    assert!(message.contains("did not finish within 1s"), "{message}");
    assert!(message.contains("gpu process died"), "{message}");
    let child = std::fs::read_to_string(&child_pid).expect("child pid");
    let alive = std::process::Command::new("kill")
        .args(["-0", child.trim()])
        .status()
        .expect("kill -0");
    assert!(
        !alive.success(),
        "the browser's child {} outlived the deadline",
        child.trim()
    );
    std::fs::remove_dir_all(&dir).expect("cleanup");
}
