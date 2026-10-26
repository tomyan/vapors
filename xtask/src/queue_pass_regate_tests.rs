//! A pass that re-gates, backs off or skips, driven through the recording
//! fake (`queue_fake.rs`).

use std::collections::{BTreeMap, VecDeque};

use super::State;
use super::tests::{ready, run_pass, two_ready};
use crate::queue_config::MergeOn;
use crate::queue_decide::Gate;
use crate::queue_fake::{Fake, clear};

#[test]
fn a_ready_pr_behind_main_is_rebased_pushed_and_gated() {
    let fake = Fake {
        rebased: Some("h1new".into()),
        gate_posts: true,
        ..two_ready()
    };
    fake.contains
        .borrow_mut()
        .insert("h1".into(), VecDeque::from([false]));
    fake.heads
        .borrow_mut()
        .insert(1, VecDeque::from(["h1".to_owned(), "h1new".to_owned()]));
    let mut state = State::default();
    run_pass(&fake, MergeOn::Gate, &mut state, false);
    let calls = fake.calls();
    let order: Vec<&String> = calls
        .iter()
        .filter(|c| c.starts_with("push") || c.starts_with("gate"))
        .collect();
    assert_eq!(order, ["push b1 h1 h1new", "gate 1 h1new"], "{calls:?}");
    assert!(!state.backed_off("h1new", 1_000));
}

#[test]
fn a_gate_that_posts_nothing_is_backed_off_and_skipped_next_pass() {
    let fake = two_ready();
    fake.comments.borrow_mut().insert(1, vec![clear("lead")]);
    let mut state = State::default();
    let log = run_pass(&fake, MergeOn::Gate, &mut state, false);
    assert!(
        state.backed_off("h1", 1_000) && log.iter().any(|l| l.contains("posted nothing")),
        "{log:?}"
    );
    let again = run_pass(&fake, MergeOn::Gate, &mut state, false);
    assert!(
        again
            .iter()
            .any(|l| l.contains("retried after its backoff")),
        "{again:?}"
    );
}

#[test]
fn a_factory_change_cleared_by_the_foreman_waits_for_the_lead() {
    let fake = Fake {
        touched: vec!["xtask/src/queue.rs".into()],
        ..two_ready()
    };
    fake.comments
        .borrow_mut()
        .insert(1, ready(1, "h1", "foreman"));
    let log = run_pass(&fake, MergeOn::Gate, &mut State::default(), false);
    assert!(
        !fake.called("merge 1")
            && log
                .iter()
                .any(|l| l.contains("needs a clear signed (lead)")),
        "{log:?}"
    );
}

#[test]
fn a_hold_posted_between_judging_and_acting_stops_the_action() {
    let fake = Fake {
        hold_after_first_read: Some(1),
        ..two_ready()
    };
    run_pass(&fake, MergeOn::Gate, &mut State::default(), false);
    assert!(
        !fake.called("merge 1") && fake.called("merge 2"),
        "{:?}",
        fake.calls()
    );
}

#[test]
fn a_dry_run_acts_on_nothing() {
    let fake = two_ready();
    let log = run_pass(&fake, MergeOn::Gate, &mut State::default(), true);
    assert!(
        !fake.called("merge") && !fake.called("gate") && !fake.called("push"),
        "{:?}",
        fake.calls()
    );
    assert!(
        log.iter().any(|l| l == "dry run: would merge #1"),
        "{log:?}"
    );
}

#[test]
fn checks_still_running_move_on_to_the_next_pr() {
    let fake = Fake {
        checks: BTreeMap::from([(1, Gate::None)]),
        ..two_ready()
    };
    let log = run_pass(&fake, MergeOn::Checks, &mut State::default(), false);
    assert!(
        fake.called("merge 2 h2") && log.iter().any(|l| l == "#1's checks are running"),
        "{log:?}"
    );
}
