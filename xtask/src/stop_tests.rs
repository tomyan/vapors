//! Stopping a child: fake processes in `sh` that handle SIGTERM by
//! removing a lock file, and that ignore it.

use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::{Duration, Instant};

use super::*;

/// A scratch lock path for one test.
fn lock_path(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("xtask-stop-{name}-{}", std::process::id()));
    let _ = std::fs::remove_file(&path);
    path
}

/// `sh` running `script` with `$LOCK` set, once it has created the lock.
fn spawn_holding(lock: &Path, script: &str) -> Child {
    let child = Command::new("sh")
        .args(["-c", script])
        .env("LOCK", lock)
        .spawn()
        .expect("sh starts");
    let started = Instant::now();
    while !lock.exists() {
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "the fake never created its lock"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    child
}

/// Like Xvfb: removes its lock on SIGTERM.
const CLEANS_UP_ON_TERM: &str =
    r#"trap 'rm -f "$LOCK"; exit 0' TERM; touch "$LOCK"; while :; do sleep 0.05; done"#;

/// Ignores SIGTERM; only SIGKILL stops it.
const IGNORES_TERM: &str = r#"trap '' TERM; touch "$LOCK"; while :; do sleep 0.05; done"#;

#[test]
fn a_process_that_handles_term_exits_and_leaves_no_lock_behind() {
    // Given
    let lock = lock_path("clean");
    let mut child = spawn_holding(&lock, CLEANS_UP_ON_TERM);

    // When
    let stopped = terminate(&mut child, Duration::from_secs(2)).expect("signals are sent");

    // Then
    assert_eq!(stopped, Stopped::Exited);
    assert!(!lock.exists(), "lock left behind at {}", lock.display());
}

#[test]
fn a_process_that_ignores_term_is_killed_at_the_deadline() {
    // Given
    let lock = lock_path("ignores");
    let mut child = spawn_holding(&lock, IGNORES_TERM);
    let started = Instant::now();

    // When
    let stopped = terminate(&mut child, Duration::from_millis(300)).expect("signals are sent");

    // Then
    assert_eq!(stopped, Stopped::Killed);
    assert!(started.elapsed() >= Duration::from_millis(300));
    assert!(
        child.try_wait().expect("waitable").is_some(),
        "the child is gone"
    );
    let _ = std::fs::remove_file(&lock);
}
