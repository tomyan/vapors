//! `cargo xtask ci` shares the gate lock; the gate excludes it.

use std::time::{Duration, Instant};

use super::*;
use crate::gate_lock::acquire;
use crate::lock_scratch::dir;

const BRIEF: Duration = Duration::from_millis(300);

#[test]
fn two_ci_runs_share_the_lock() {
    let dir = dir("shared-two");
    let first = acquire_shared(&dir, "/w/first", BRIEF);
    let second = acquire_shared(&dir, "/w/second", BRIEF);
    assert!(first.is_ok() && second.is_ok(), "{first:?} {second:?}");
}

#[test]
fn a_ci_run_waits_for_a_running_gate_then_fails_naming_it() {
    // Given
    let dir = dir("shared-gate");
    let _gate = acquire(&dir, "/w/gating", BRIEF).expect("gate");

    // When
    let started = Instant::now();
    let refusal = acquire_shared(&dir, "/w/ci", BRIEF).expect_err("waits");

    // Then
    assert!(started.elapsed() >= BRIEF, "it waited");
    assert!(refusal.contains("/w/gating"), "{refusal}");
}

#[test]
fn a_gate_waits_for_a_running_ci_then_fails_naming_its_worktree() {
    // Given
    let dir = dir("shared-ci");
    let _ci = acquire_shared(&dir, "/w/checking", BRIEF).expect("ci");

    // When
    let refusal = acquire(&dir, "/w/gate", BRIEF).expect_err("waits");

    // Then
    assert!(refusal.contains("ci in /w/checking"), "{refusal}");
}

#[test]
fn a_waiting_gate_goes_before_a_ci_run_that_comes_later() {
    // Given: a ci run holds the lock, and a gate starts waiting for it
    let dir = dir("shared-turn");
    let first = acquire_shared(&dir, "/w/first", BRIEF).expect("ci");
    let gate_dir = dir.to_path_buf();
    let gate = std::thread::spawn(move || {
        acquire(&gate_dir, "/w/gate", Duration::from_secs(5)).map(|_| ())
    });
    turn_taken(&dir);

    // When: a later ci run asks while the gate waits
    let later = acquire_shared(&dir, "/w/later", BRIEF);
    drop(first);

    // Then
    assert!(later.is_err(), "the later ci run went first");
    assert_eq!(gate.join().expect("joined"), Ok(()));
}

#[test]
fn a_ci_run_s_marker_goes_when_it_ends() {
    // Given
    let dir = dir("shared-marker");
    let held = acquire_shared(&dir, "/w/ci", BRIEF).expect("ci");
    let marker = dir.join(format!("ci.{}", std::process::id()));
    assert!(std::fs::read_to_string(&marker).is_ok_and(|t| t.contains("/w/ci")));

    // When
    drop(held);

    // Then
    assert!(!marker.exists());
}

/// A pid above Linux's largest `pid_max` (2^22), so never a live process.
const GONE: u32 = 4_194_305;

#[test]
fn a_stale_gate_holder_is_removed_and_not_named() {
    // Given: a killed gate left its holder file, and a ci run holds a share
    let dir = dir("shared-stale");
    let _ci = acquire_shared(&dir, "/w/live-ci", BRIEF).expect("ci");
    std::fs::write(
        dir.join(crate::gate_lock::HOLDER),
        format!("pid {GONE} in /w/killed since 1\n"),
    )
    .expect("holder");

    // When
    let refusal = acquire(&dir, "/w/gate", BRIEF).expect_err("waits");

    // Then
    assert!(refusal.contains("ci in /w/live-ci"), "{refusal}");
    assert!(!refusal.contains(&GONE.to_string()), "{refusal}");
    assert!(!dir.join(crate::gate_lock::HOLDER).exists());
}

#[test]
fn a_ci_holder_is_named_with_how_long_it_has_held_its_share() {
    let dir = dir("shared-age");
    let _ci = acquire_shared(&dir, "/w/ci", BRIEF).expect("ci");
    let holders = ci_holders(&dir);
    let start = format!("ci in /w/ci (pid {}) for ", std::process::id());
    assert!(
        holders.len() == 1 && holders[0].starts_with(&start) && holders[0].ends_with(" s"),
        "{holders:?}"
    );
}

#[test]
fn a_marker_older_than_its_process_names_a_reused_pid_and_is_left_out() {
    // Given: a marker for this pid stamped in 1970, before this process started
    let dir = dir("shared-reused");
    std::fs::create_dir_all(&*dir).expect("dir");
    let me = std::process::id();
    std::fs::write(
        dir.join(format!("ci.{me}")),
        format!("pid {me} in /w/old since 1000\n"),
    )
    .expect("marker");

    // Then
    assert_eq!(ci_holders(&dir), Vec::<String>::new());
}

/// Wait until another holder has `gate.turn`: poll its lock, rather than
/// sleep for a time a loaded box may not keep (5 s at most).
fn turn_taken(dir: &std::path::Path) {
    let turn = crate::gate_lock::open(dir, crate::gate_lock::TURN).expect("turn");
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(5) {
        match turn.try_lock() {
            Ok(()) => {
                let _ = turn.unlock();
                std::thread::sleep(Duration::from_millis(5));
            }
            Err(_) => return,
        }
    }
    panic!("the gate did not take gate.turn within 5 s");
}
