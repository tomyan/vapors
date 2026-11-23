//! Gates' holds across processes: each test starts a second process
//! (`lock_child.rs`) and sequences it by its lines and its stdin.

use std::time::Duration;

use super::*;
use crate::lock_child_handle::{Child, turn_taken};
use crate::scratch::lock_dir;

/// A child's own waits: never reached when the holds work, since every
/// test releases what the child waits for; a child that reaches it says
/// `refused`, which fails the test.
const CHILD_WAIT: Duration = Duration::from_secs(15);
/// This process's waits where the test expects a refusal.
const BRIEF: Duration = Duration::from_millis(300);

fn gate(dir: &std::path::Path, worktree: &str) -> GateHolds {
    GateHolds::new(Place::brief(dir, worktree, BRIEF))
}

#[test]
fn two_gates_shared_steps_run_at_the_same_time() {
    // Given
    let dir = lock_dir("process-two-shared");
    let first = Child::start("share", &dir, "/w/first", CHILD_WAIT, &[]);
    first.expect("held");

    // When
    let second = Child::start("share", &dir, "/w/second", CHILD_WAIT, &[]);

    // Then
    second.expect("held");
    first.release();
    second.release();
}

#[test]
fn an_exclusive_step_waits_for_running_shares_then_goes() {
    // Given
    let dir = lock_dir("process-exclusive-waits");
    let sharing = Child::start("share", &dir, "/w/sharing", CHILD_WAIT, &[]);
    sharing.expect("held");
    let alone = Child::start("exclusive", &dir, "/w/alone", CHILD_WAIT, &[]);
    alone.expect_waiting_on("/w/sharing");
    assert!(alone.said_nothing(), "it went beside a share");

    // When
    sharing.release();

    // Then
    alone.expect("held");
    alone.release();
}

#[test]
fn no_new_share_starts_while_an_exclusive_step_waits_or_holds() {
    // Given: a share, and an exclusive step waiting for it
    let dir = lock_dir("process-no-new-share");
    let sharing = Child::start("share", &dir, "/w/sharing", CHILD_WAIT, &[]);
    sharing.expect("held");
    let alone = Child::start("exclusive", &dir, "/w/alone", CHILD_WAIT, &[]);
    turn_taken(&dir);

    // When / Then: waiting
    assert!(
        gate(&dir, "/w/late").hold(Hold::Shared).is_err(),
        "a share went first"
    );
    sharing.release();
    alone.expect("held");

    // When / Then: holding
    let refusal = gate(&dir, "/w/late").hold(Hold::Shared).expect_err("held");
    assert!(refusal.contains("/w/alone"), "{refusal}");
    alone.release();
    assert_eq!(gate(&dir, "/w/late").hold(Hold::Shared), Ok(()));
}

#[test]
fn a_gate_killed_during_its_exclusive_step_leaves_nothing_held() {
    // Given
    let dir = lock_dir("process-killed");
    let killed = Child::start("exclusive", &dir, "/w/killed", CHILD_WAIT, &[]);
    killed.expect("held");

    // When
    killed.kill();

    // Then: its quiet holder file is left, and read as ended
    assert!(
        dir.join(quiet_lock::FILE).exists(),
        "nothing was left to recover"
    );
    assert_eq!(gate(&dir, "/w/next").hold(Hold::Shared), Ok(()));
}

#[test]
fn two_gates_switching_to_exclusive_at_once_each_get_their_turn() {
    // Given: two gates in their shared steps
    let dir = lock_dir("process-switch");
    let mut first = Child::start("switch", &dir, "/w/first", CHILD_WAIT, &[]);
    let mut second = Child::start("switch", &dir, "/w/second", CHILD_WAIT, &[]);
    first.expect("shared");
    second.expect("shared");

    // When: both go to an exclusive step together
    first.send("go");
    second.send("go");

    // Then: one holds it, and the other once the first ends
    let (went, waiting) = first_to_go(first, second);
    assert!(waiting.said_nothing(), "both held it");
    went.release();
    waiting.expect("exclusive");
    waiting.release();
}

/// The child that says `exclusive` first, and the other; polled, since
/// either may, until [`crate::lock_child_handle::WITHIN`].
fn first_to_go(first: Child, second: Child) -> (Child, Child) {
    let started = std::time::Instant::now();
    while started.elapsed() < crate::lock_child_handle::WITHIN {
        if let Some(line) = first.line_now() {
            assert_eq!(line, "exclusive");
            return (first, second);
        }
        if let Some(line) = second.line_now() {
            assert_eq!(line, "exclusive");
            return (second, first);
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("neither gate took its exclusive step: they deadlocked");
}

#[test]
fn an_old_protocol_gate_keeps_new_gates_out_until_it_ends() {
    // Given: a gate built before holds were per step, holding the quiet
    // lock and the gate lock exclusive for its whole run
    let dir = lock_dir("process-old-gate");
    let old = Child::start("old-gate", &dir, "/w/old", CHILD_WAIT, &[]);
    old.expect("held");

    // When
    let shared = gate(&dir, "/w/new").hold(Hold::Shared);
    let exclusive = gate(&dir, "/w/new").hold(Hold::Exclusive);

    // Then
    for refused in [shared, exclusive] {
        let refusal = refused.expect_err("went beside the old gate");
        assert!(refusal.contains("/w/old"), "{refusal}");
    }
    old.release();
    assert_eq!(gate(&dir, "/w/new").hold(Hold::Shared), Ok(()));
}

#[test]
fn an_old_protocol_gate_between_its_quiet_lock_and_its_gate_lock_keeps_new_shares_out() {
    // Given: its quiet lock taken, its gate lock not yet
    let dir = lock_dir("process-old-quiet");
    let old = Child::start("old-quiet", &dir, "/w/old", CHILD_WAIT, &[]);
    old.expect("held");

    // When
    let refusal = gate(&dir, "/w/new")
        .hold(Hold::Shared)
        .expect_err("went beside it");

    // Then
    assert!(
        refusal.contains("the quiet lock") && refusal.contains("/w/old"),
        "{refusal}"
    );
    old.release();
}

#[test]
fn an_xtask_test_run_by_a_gate_s_step_goes_on_beside_its_gate_s_exclusive_hold() {
    // Given
    let dir = lock_dir("process-nested");
    let mut gating = gate(&dir, "/w/gate");
    gating.hold(Hold::Exclusive).expect("exclusive");

    // When
    let nested = Child::start(
        "nested-test",
        &dir,
        "/w/gate",
        BRIEF,
        &[("XTASK_GATE", "1")],
    );
    let outside = Child::start("nested-test", &dir, "/w/other", BRIEF, &[]);

    // Then: the step's run goes on; a run outside the gate waits on it
    nested.expect("went on");
    let line = outside.next_line().unwrap_or_default();
    assert!(
        line.starts_with("refused: ") && line.contains("/w/gate"),
        "{line}"
    );
}
