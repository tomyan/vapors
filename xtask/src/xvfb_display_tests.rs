//! The display search over fake `/tmp` trees: lock files, sockets, an
//! injected liveness check and user id.

use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;

use super::*;

/// An empty fake `/tmp` with its `.X11-unix` directory.
fn fake_tmp(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("xtask-xvfb-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join(".X11-unix")).expect("create the fake /tmp");
    root
}

/// Xvfb's lock for display `n`, holding `pid` as Xvfb writes it, and
/// its socket.
fn lock(root: &Path, n: u32, pid: u32) {
    fs::write(root.join(format!(".X{n}-lock")), format!("{pid:>10}\n")).expect("write lock");
    fs::write(root.join(format!(".X11-unix/X{n}")), "").expect("write socket");
}

fn my_uid(root: &Path) -> u32 {
    fs::metadata(root).expect("the fake /tmp exists").uid()
}

fn search(root: &Path, uid: u32, live: &[u32]) -> Result<Option<Claimed>, String> {
    let is_running = |pid: u32| live.contains(&pid);
    let displays = DisplaySearch {
        root,
        uid,
        is_running: &is_running,
    };
    displays.claim(99..=101)
}

#[test]
fn the_first_display_without_a_lock_or_socket_is_free() {
    // Given
    let root = fake_tmp("empty");

    // When
    let claimed = search(&root, my_uid(&root), &[]).expect("readable");

    // Then
    assert_eq!(
        claimed,
        Some(Claimed {
            number: 99,
            removed: vec![]
        })
    );
}

#[test]
fn a_lock_held_by_a_running_process_stays_taken() {
    // Given
    let root = fake_tmp("live");
    lock(&root, 99, 4242);

    // When
    let claimed = search(&root, my_uid(&root), &[4242]).expect("readable");

    // Then
    assert_eq!(claimed.map(|c| c.number), Some(100));
    assert!(root.join(".X99-lock").exists());
}

#[test]
fn a_lock_of_a_dead_process_is_removed_with_its_socket_and_reported() {
    // Given
    let root = fake_tmp("dead");
    lock(&root, 99, 4242);

    // When
    let claimed = search(&root, my_uid(&root), &[]).expect("readable");

    // Then
    let claimed = claimed.expect("display 99 is free again");
    assert_eq!(claimed.number, 99);
    assert!(!root.join(".X99-lock").exists());
    assert!(!root.join(".X11-unix/X99").exists());
    assert_eq!(claimed.removed.len(), 1);
    assert!(
        claimed.removed[0].contains(".X99-lock"),
        "{:?}",
        claimed.removed
    );
    assert!(claimed.removed[0].contains("4242"), "{:?}", claimed.removed);
}

#[test]
fn a_dead_lock_owned_by_another_user_stays_taken() {
    // Given
    let root = fake_tmp("other-user");
    lock(&root, 99, 4242);

    // When
    let claimed = search(&root, my_uid(&root) + 1, &[]).expect("readable");

    // Then
    assert_eq!(claimed.map(|c| c.number), Some(100));
    assert!(root.join(".X99-lock").exists());
}

#[test]
fn a_socket_without_a_lock_stays_taken() {
    // Given
    let root = fake_tmp("socket-only");
    fs::write(root.join(".X11-unix/X99"), "").expect("write socket");

    // When
    let claimed = search(&root, my_uid(&root), &[]).expect("readable");

    // Then
    assert_eq!(claimed.map(|c| c.number), Some(100));
}

#[test]
fn every_display_taken_by_a_live_process_is_none() {
    // Given
    let root = fake_tmp("full");
    for n in 99..=101 {
        lock(&root, n, 7000 + n);
    }

    // When
    let claimed = search(&root, my_uid(&root), &[7099, 7100, 7101]).expect("readable");

    // Then
    assert_eq!(claimed, None);
}

#[test]
fn a_lock_without_a_pid_is_refused_naming_the_file() {
    // Given
    let root = fake_tmp("garbled");
    fs::write(root.join(".X99-lock"), "not a pid\n").expect("write lock");

    // When
    let refused = search(&root, my_uid(&root), &[]).expect_err("unreadable lock");

    // Then
    assert!(refused.contains(".X99-lock"), "{refused}");
    assert!(refused.contains("not a pid"), "{refused}");
}

#[test]
fn a_lock_that_cannot_be_read_is_refused_naming_the_file() {
    // Given: a directory where the lock file should be
    let root = fake_tmp("unreadable");
    fs::create_dir(root.join(".X99-lock")).expect("create dir");

    // When
    let refused = search(&root, my_uid(&root), &[]).expect_err("unreadable lock");

    // Then
    assert!(refused.contains(".X99-lock"), "{refused}");
}
