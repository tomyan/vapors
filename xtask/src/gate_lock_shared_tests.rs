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
    std::thread::sleep(Duration::from_millis(200));

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
