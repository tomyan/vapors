//! A gate that fails to finish or refuses before posting, driven through
//! the recording fake (`queue_fake.rs`).

use super::State;
use super::tests::{run_pass, two_ready};
use crate::queue_config::MergeOn;
use crate::queue_fake::{Fake, clear};

/// Two PRs cleared by the lead, neither gated yet.
fn ungated() -> Fake {
    let fake = two_ready();
    fake.comments.borrow_mut().insert(1, vec![clear("lead")]);
    fake
}

#[test]
fn a_gate_that_errs_is_backed_off_and_said() {
    // Given: the gate is killed at its deadline
    let fake = Fake {
        gate_fails: Some("the gate did not finish within 3600s; killed".into()),
        ..ungated()
    };
    let mut state = State::default();

    // When
    let log = run_pass(&fake, MergeOn::Gate, &mut state, false);

    // Then
    assert!(state.backed_off("h1", 1_000), "{log:?}");
    assert!(
        log.iter().any(|l| l
            == "FAILED #1: the gate did not finish within 3600s; killed; h1 backed off for 30 min"),
        "{log:?}"
    );
}

#[test]
fn a_gate_that_exits_without_posting_logs_its_last_lines() {
    // Given: gate --pr refuses before gating and posts nothing
    let fake = Fake {
        gate_exit: 1,
        gate_tail: "gate --pr: reading #1\nFAIL gate --pr: gh did not answer within 30s".into(),
        ..ungated()
    };

    // When
    let log = run_pass(&fake, MergeOn::Gate, &mut State::default(), false);

    // Then
    let at = log
        .iter()
        .position(|l| l == "#1's gate posted nothing at h1; backed off for 30 min; its last lines:")
        .unwrap_or_else(|| panic!("{log:?}"));
    assert_eq!(
        log[at + 1..at + 3],
        [
            "  gate --pr: reading #1",
            "  FAIL gate --pr: gh did not answer within 30s"
        ]
    );
}
