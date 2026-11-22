//! A gate's hold within one process: kept while steps need the same one,
//! released before another is taken.

use std::time::Duration;

use super::*;
use crate::scratch::lock_dir;

const BRIEF: Duration = Duration::from_millis(300);

#[test]
fn a_gate_going_from_shared_to_exclusive_does_not_wait_on_its_own_share() {
    // Given
    let dir = lock_dir("hold-up");
    let mut holds = GateHolds::new(Place::brief(&dir, "/w/gate", BRIEF));
    holds.hold(Hold::Shared).expect("shared");

    // When
    let exclusive = holds.hold(Hold::Exclusive);

    // Then
    assert_eq!(exclusive, Ok(()));
}

#[test]
fn a_gate_s_exclusive_hold_is_the_quiet_lock_as_a_gate_and_goes_with_it() {
    // Given
    let dir = lock_dir("hold-quiet");
    let mut holds = GateHolds::new(Place::brief(&dir, "/w/gate", BRIEF));

    // When
    holds.hold(Hold::Exclusive).expect("exclusive");
    let during = quiet_lock::current(&dir).expect("read");
    holds.hold(Hold::Shared).expect("shared");
    let after = quiet_lock::current(&dir).expect("read");

    // Then
    let during = during.expect("held");
    assert_eq!(
        (during.form, during.owner.worktree.as_str()),
        (Form::Gate, "/w/gate")
    );
    assert_eq!(after, None);
}

#[test]
fn a_gate_s_shared_hold_keeps_its_exclusive_hold_from_another_gate() {
    // Given
    let dir = lock_dir("hold-shared-then-other");
    let mut gate = GateHolds::new(Place::brief(&dir, "/w/sharing", BRIEF));
    gate.hold(Hold::Shared).expect("shared");

    // When
    let mut other = GateHolds::new(Place::brief(&dir, "/w/other", BRIEF));
    let refusal = other.hold(Hold::Exclusive).expect_err("waits");

    // Then
    assert!(refusal.contains("/w/sharing"), "{refusal}");
}

#[test]
fn under_a_quiet_hold_from_its_own_worktree_a_gate_s_exclusive_hold_takes_no_quiet_lock() {
    // Given
    let dir = lock_dir("hold-own-quiet");
    let mine = Holder::me("/w/gate", Duration::from_mins(5), Form::Hold, "hold");
    let _quiet = quiet_lock::take(&dir, mine.clone(), BRIEF).expect("hold");
    let mut holds = GateHolds::new(Place::brief(&dir, "/w/gate", BRIEF));

    // When
    holds.hold(Hold::Exclusive).expect("exclusive");

    // Then
    assert_eq!(quiet_lock::current(&dir), Ok(Some(mine)));
}

/// How many times [`counted_load`] ran in this process (nextest runs
/// each test in a process of its own).
static LOADS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

#[allow(
    clippy::unnecessary_wraps,
    reason = "a stand-in for the load wait, whose signature `Place::load` fixes"
)]
fn counted_load(_: &str) -> Result<(), String> {
    LOADS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    Ok(())
}

#[test]
fn an_exclusive_step_waits_for_the_load_only_once_it_holds_the_gate_lock() {
    // Given: another gate holds the lock exclusive
    let dir = lock_dir("hold-load-after");
    let other = gate_lock::acquire(&dir, "/w/other", BRIEF).expect("other");
    let place = Place {
        load: counted_load,
        ..Place::brief(&dir, "/w/gate", BRIEF)
    };
    let mut holds = GateHolds::new(place);

    // When
    let waited = holds.hold(Hold::Exclusive);
    let loads_while_held_elsewhere = LOADS.load(std::sync::atomic::Ordering::SeqCst);
    drop(other);
    holds.hold(Hold::Exclusive).expect("exclusive");

    // Then
    assert!(waited.is_err(), "it went beside the other gate");
    assert_eq!(
        loads_while_held_elsewhere, 0,
        "it read the load before it held the lock"
    );
    assert_eq!(LOADS.load(std::sync::atomic::Ordering::SeqCst), 1);
}
