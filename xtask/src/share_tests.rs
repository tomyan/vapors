//! Who takes a ci slot with its share, and a share inside a gate's step.

use std::time::Duration;

use super::*;
use crate::gate_lock_slots::SLOTS;
use crate::scratch::lock_dir;

const BRIEF: Duration = Duration::from_millis(300);

/// Every ci slot taken by ci runs in other worktrees.
fn slots_full(dir: &std::path::Path) -> Vec<Shared> {
    (0..SLOTS)
        .map(|n| {
            take(&Place::brief(dir, &format!("/w/ci-{n}"), BRIEF), Sharer::Ci).expect("a slot")
        })
        .collect()
}

#[test]
fn a_test_run_takes_its_share_while_every_ci_slot_is_taken() {
    // Given
    let dir = lock_dir("share-test-no-slot");
    let _ci = slots_full(&dir);

    // When
    let test = take(&Place::brief(&dir, "/w/test", BRIEF), Sharer::Test);

    // Then
    assert!(test.is_ok(), "{test:?}");
}

#[test]
fn a_gate_s_shared_steps_wait_for_a_ci_slot_and_name_its_holders() {
    // Given
    let dir = lock_dir("share-gate-slot");
    let _ci = slots_full(&dir);

    // When
    let refusal = take(&Place::brief(&dir, "/w/gate", BRIEF), Sharer::Gate).expect_err("no slot");

    // Then
    assert!(
        refusal.contains("ci slots") && refusal.contains("/w/ci-0"),
        "{refusal}"
    );
}

#[test]
fn inside_a_gate_s_step_a_run_takes_no_share_and_reads_no_place() {
    // When
    let taken = unless_in_gate_at(Sharer::Test, Some("1"), || panic!("read the place"));

    // Then
    assert!(matches!(taken, Ok(None)), "{taken:?}");
}

#[test]
fn outside_a_gate_a_test_run_holds_a_share_that_keeps_a_gate_s_exclusive_step_out() {
    // Given
    let dir = lock_dir("share-test-holds");
    let place = Place::brief(&dir, "/w/test", BRIEF);

    // When
    let share = unless_in_gate_at(Sharer::Test, None, || Ok(place.clone())).expect("taken");

    // Then
    assert!(share.is_some());
    let refusal = crate::gate_lock::acquire(&dir, "/w/gate", BRIEF).expect_err("excluded");
    assert!(refusal.contains("/w/test"), "{refusal}");
}

#[test]
fn a_gate_s_share_is_named_as_a_gate_to_whoever_waits_on_it() {
    // Given
    let dir = lock_dir("share-gate-named");
    let _gate = take(&Place::brief(&dir, "/w/gating", BRIEF), Sharer::Gate).expect("share");

    // When
    let refusal = crate::gate_lock::acquire(&dir, "/w/alone", BRIEF).expect_err("excluded");

    // Then
    assert!(refusal.contains("gate in /w/gating (pid "), "{refusal}");
}

/// The lock directory [`quiet_taken_meanwhile`] writes into.
static TAKEN_IN: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();

/// A load wait during which another worktree takes the quiet lock for
/// 200 ms, as a `quiet` started between a run's quiet wait and its share.
fn quiet_taken_meanwhile(_: &str) -> Result<(), String> {
    let dir = TAKEN_IN.get().expect("the test's lock directory");
    let holder = Holder::me(
        "/w/timed",
        Duration::from_millis(200),
        crate::quiet_lock::Form::Hold,
        "x",
    );
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    std::fs::write(dir.join(crate::quiet_lock::FILE), holder.line()).map_err(|e| e.to_string())
}

#[test]
fn a_share_taken_as_the_quiet_lock_was_taken_is_let_go_and_waited_for_again() {
    // Given: the quiet lock is taken during every round's load wait
    let dir = lock_dir("share-recheck");
    TAKEN_IN.set(dir.to_path_buf()).expect("set once");
    let place = Place {
        load: quiet_taken_meanwhile,
        quiet_wait: Duration::from_secs(5),
        ..Place::brief(&dir, "/w/ci", BRIEF)
    };

    // When
    let refusal = take(&place, Sharer::Ci).expect_err("taken every round");

    // Then: it gave up after its rounds, holding no share
    assert!(refusal.contains("taken 3 times"), "{refusal}");
    assert!(
        crate::gate_lock::acquire(&dir, "/w/after", BRIEF).is_ok(),
        "a share was kept"
    );
}
