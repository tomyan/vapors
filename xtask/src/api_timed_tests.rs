use std::process::Command;
use std::time::{Duration, Instant};

use super::{git_within, within};

#[test]
fn a_child_that_ends_in_time_gives_its_streams_apart() {
    let mut command = Command::new("sh");
    command.args(["-c", "echo out; echo err >&2; exit 3"]);
    let ended = within(command, Duration::from_secs(20)).expect("ended");
    assert_eq!(
        (ended.stdout, ended.stderr),
        (b"out\n".to_vec(), b"err\n".to_vec())
    );
    assert_eq!(ended.status.code(), Some(3));
}

#[test]
fn what_a_child_started_is_killed_with_it_at_the_deadline() {
    // Given: a child that starts a grandchild, which would write a
    // marker after 1 s
    let dir = tempfile::tempdir().expect("a scratch directory");
    let marker = dir.path().join("finished");
    let mut command = Command::new("sh");
    command.args([
        "-c",
        &format!("(sleep 1; touch {}) & wait", marker.display()),
    ]);

    // When
    let refused = within(command, Duration::from_millis(100))
        .err()
        .expect("killed");
    std::thread::sleep(Duration::from_millis(1500));

    // Then
    assert!(refused.contains("passed its deadline"), "{refused}");
    assert!(!marker.exists(), "the grandchild outlived the deadline");
}

#[test]
fn a_child_past_its_deadline_is_killed_and_named() {
    let mut command = Command::new("sleep");
    command.arg("30");
    let started = Instant::now();
    let refused = within(command, Duration::from_millis(100))
        .err()
        .expect("killed");
    assert!(
        refused.starts_with("sleep passed its deadline"),
        "{refused}"
    );
    assert!(
        started.elapsed() < Duration::from_secs(20),
        "{:?}",
        started.elapsed()
    );
}

#[test]
fn git_in_a_directory_gives_its_answer_or_its_error() {
    let dir = tempfile::tempdir().expect("a scratch directory");
    let deadline = Duration::from_secs(20);
    assert!(git_within(dir.path(), &["rev-parse", "HEAD"], deadline).is_err());
    assert_eq!(
        git_within(dir.path(), &["--version"], deadline).map(|v| v.starts_with("git version")),
        Ok(true)
    );
}
