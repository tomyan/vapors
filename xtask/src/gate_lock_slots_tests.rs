//! At most [`SLOTS`] ci runs at once; the next waits for a slot.

use std::time::{Duration, Instant};

use crate::gate_lock_shared::acquire_shared;
use crate::scratch::lock_dir;

use super::*;

const BRIEF: Duration = Duration::from_millis(300);

#[test]
fn three_ci_runs_go_together() {
    // Given
    let dir = lock_dir("slots-three");

    // When
    let held: Vec<_> = ["/w/a", "/w/b", "/w/c"]
        .iter()
        .map(|worktree| acquire_shared(&dir, worktree, BRIEF))
        .collect();

    // Then
    assert!(held.iter().all(Result::is_ok), "{held:?}");
}

#[test]
fn a_fourth_ci_run_waits_then_fails_naming_the_three_that_hold_the_slots() {
    // Given
    let dir = lock_dir("slots-fourth");
    let _held: Vec<_> = ["/w/a", "/w/b", "/w/c"]
        .iter()
        .map(|worktree| acquire_shared(&dir, worktree, BRIEF).expect("a slot"))
        .collect();

    // When
    let started = Instant::now();
    let refusal = acquire_shared(&dir, "/w/d", BRIEF).expect_err("no slot");

    // Then
    let waited = started.elapsed();
    assert!(waited >= BRIEF && waited < 10 * BRIEF, "waited {waited:?}");
    assert!(
        refusal.contains(&format!("all {SLOTS} ci slots")),
        "{refusal}"
    );
    // One process here, so one `ci.<pid>` marker: the last run's.
    assert!(refusal.contains("by ci in /w/c"), "{refusal}");
    assert!(refusal.contains("fix:"), "{refusal}");
}

#[test]
fn a_waiting_ci_run_takes_the_slot_a_finished_one_frees() {
    // Given
    let dir = lock_dir("slots-freed");
    let mut held: Vec<_> = ["/w/a", "/w/b", "/w/c"]
        .iter()
        .map(|worktree| acquire_shared(&dir, worktree, BRIEF).expect("a slot"))
        .collect();
    let waiting_dir = dir.to_path_buf();
    let waiting = std::thread::spawn(move || {
        acquire_shared(&waiting_dir, "/w/d", Duration::from_secs(5)).map(|_| ())
    });

    // When
    std::thread::sleep(Duration::from_millis(200));
    held.pop();

    // Then
    assert_eq!(waiting.join().expect("joined"), Ok(()));
}
