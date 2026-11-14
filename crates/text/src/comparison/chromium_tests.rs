//! The runner's checks on plain data: stale files and sizes.

use super::*;

#[test]
fn a_stale_screenshot_is_removed_and_a_missing_one_is_fine() {
    // Given
    let scratch = tempfile::Builder::new()
        .prefix("vapors-chromium-")
        .tempdir();
    let scratch = scratch.expect("a scratch directory under the temp dir");
    let dir = scratch.path();
    let png = dir.join("stale.png");
    std::fs::write(&png, b"old").expect("write");

    // When
    remove_stale(&png).expect("removed");
    let again = remove_stale(&png);

    // Then
    assert!(!png.exists());
    assert_eq!(again, Ok(()));
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
#[allow(clippy::too_many_lines)] // one scenario: the hung browser, its child and its stderr
fn a_browser_past_its_deadline_is_killed_with_its_children_and_its_stderr_reported() {
    // Given: a "browser" that writes to stderr, leaves a child holding
    // the pipe, then hangs
    let scratch = tempfile::Builder::new()
        .prefix("vapors-chromium-hang-")
        .tempdir();
    let scratch = scratch.expect("a scratch directory under the temp dir");
    let dir = scratch.path();
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

    // Then: the 1 s deadline, then naming the browser, whose `--version`
    // hangs too and is cut off at its own 10 s. Each failure says how
    // long the run took and what the fake browser had done by then, so
    // a slow step under load is named
    let elapsed = started.elapsed();
    let message = error.to_string();
    let stage = |what: &str| {
        format!(
            "{what} after {elapsed:?}; the launcher's message: {message}; the child's pid file: {:?}",
            std::fs::read_to_string(&child_pid).ok()
        )
    };
    assert!(
        elapsed < std::time::Duration::from_secs(20),
        "{}",
        stage("the kill was late")
    );
    assert!(
        message.contains("did not finish within 1s"),
        "{}",
        stage("no deadline message")
    );
    assert!(
        message.contains("version unknown"),
        "{}",
        stage("the browser not named")
    );
    assert!(
        message.contains("gpu process died"),
        "{}",
        stage("no stderr: the fake browser's shell had not reached its first line")
    );
    let child = std::fs::read_to_string(&child_pid).unwrap_or_else(|_| {
        panic!(
            "{}",
            stage("no child pid: the fake browser was killed before it started its child")
        )
    });
    let left = state_once_killed(child.trim());
    assert!(
        left.is_none(),
        "the browser's child {} outlived the deadline, in state {left:?}",
        child.trim()
    );
}

/// How long a killed process may take to end. Measured 2026-11-14, 60
/// runs 12 at a time at load 76 to 114: 59 had ended at the first read
/// (under 0.1 ms), the slowest after 20 ms. 5 s is about 250 times that.
const KILLED_ENDS_WITHIN: std::time::Duration = std::time::Duration::from_secs(5);

/// The state of process `pid` once it has had [`KILLED_ENDS_WITHIN`] to
/// end, from /proc: none when it is gone or a zombie. A killed orphan
/// stays a zombie until init reaps it, and `kill -0` counts a zombie as
/// alive, which failed this test under load.
fn state_once_killed(pid: &str) -> Option<char> {
    let started = std::time::Instant::now();
    loop {
        let state = running_state(pid);
        if state.is_none() || started.elapsed() >= KILLED_ENDS_WITHIN {
            println!("pid {pid}: {state:?} after {:?}", started.elapsed());
            return state;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

/// `pid`'s state from /proc/<pid>/stat: none when the process is gone or
/// a zombie. Any other read error or a line of another shape panics,
/// naming the pid and what was read, rather than counting as ended.
fn running_state(pid: &str) -> Option<char> {
    let path = format!("/proc/{pid}/stat");
    let stat = match std::fs::read_to_string(&path) {
        Ok(stat) => stat,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return None,
        Err(e) => panic!("{path}: {e}"),
    };
    let state = stat
        .rfind(')')
        .and_then(|end| stat[end + 1..].trim_start().chars().next())
        .unwrap_or_else(|| {
            panic!(
                "{path} reads `{}`, with no state after the name",
                stat.trim()
            )
        });
    (state != 'Z').then_some(state)
}
