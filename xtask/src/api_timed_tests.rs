use std::process::Command;
use std::time::{Duration, Instant};

use super::within;

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
