//! A pass on a short disk, and the targets a merge empties, driven through
//! the recording fake (`queue_fake.rs`).

use std::collections::VecDeque;
use std::path::PathBuf;

use super::State;
use super::tests::{run_pass, two_ready};
use crate::queue_config::MergeOn;
use crate::queue_disk::GB;
use crate::queue_fake::Fake;

/// Two ready PRs, #1 behind main for `passes` passes, on a disk with
/// 10 GB free.
fn short_disk(passes: usize) -> Fake {
    let fake = Fake {
        free: Some(10 * GB),
        largest: vec![(62 * GB, PathBuf::from("/t/w-desktop"))],
        rebased: Some("h1new".into()),
        ..two_ready()
    };
    fake.contains
        .borrow_mut()
        .insert("h1".into(), VecDeque::from(vec![false; passes]));
    fake
}

#[test]
fn a_short_disk_stops_the_gate_names_the_largest_targets_and_lets_a_merge_through() {
    // Given
    let fake = short_disk(1);

    // When
    let log = run_pass(&fake, MergeOn::Gate, &mut State::default(), false);

    // Then
    // The judge's rebase check runs; the action's push and gate do not.
    assert!(
        !fake.called("push") && !fake.called("gate"),
        "{:?}",
        fake.calls()
    );
    assert!(fake.called("merge 2 h2"), "{:?}", fake.calls());
    assert!(
        log.iter()
            .any(|l| l == "#1 is not gated: the disk has under 60 GB free"),
        "{log:?}"
    );
    assert!(
        log.iter()
            .any(|l| l
                .starts_with("the disk has 10 GB free; the largest targets: 62 GB /t/w-desktop;")),
        "{log:?}"
    );
}

#[test]
fn a_short_disk_is_said_once_and_measured_by_du_once() {
    // Given
    let fake = short_disk(2);
    let mut state = State::default();
    run_pass(&fake, MergeOn::Gate, &mut state, false);

    // When
    let again = run_pass(&fake, MergeOn::Gate, &mut state, false);

    // Then
    assert!(!again.iter().any(|l| l.contains("not gated")), "{again:?}");
    let du_runs = fake.calls().iter().filter(|c| *c == "largest").count();
    assert_eq!(du_runs, 1, "{:?}", fake.calls());
}

#[test]
fn checks_mode_updates_a_branch_whatever_the_disk_holds() {
    // Given
    let fake = Fake {
        free: Some(0),
        ..short_disk(1)
    };

    // When
    run_pass(&fake, MergeOn::Checks, &mut State::default(), false);

    // Then
    assert!(fake.called("update 1"), "{:?}", fake.calls());
    assert!(!fake.called("free"), "{:?}", fake.calls());
}

#[test]
fn a_merge_empties_its_branch_s_targets_and_logs_each() {
    // Given
    let fake = Fake {
        emptied: vec!["emptied /t/x (3 entries)".into()],
        ..two_ready()
    };

    // When
    let log = run_pass(&fake, MergeOn::Gate, &mut State::default(), false);

    // Then
    let calls = fake.calls();
    let merged = calls.iter().position(|c| c == "merge 1 h1");
    let emptied = calls.iter().position(|c| c == "empty b1");
    assert!(merged.is_some() && emptied > merged, "{calls:?}");
    assert!(
        log.iter().any(|l| l == "emptied /t/x (3 entries)"),
        "{log:?}"
    );
}

#[test]
fn a_failure_to_empty_targets_is_logged_and_the_merge_stands() {
    // Given
    let fake = Fake {
        empty_fails: true,
        ..two_ready()
    };

    // When
    let log = run_pass(&fake, MergeOn::Gate, &mut State::default(), false);

    // Then
    assert!(
        log.iter()
            .any(|l| l == "FAILED to empty b1's targets: git worktree list refused"),
        "{log:?}"
    );
    assert!(
        fake.called("delete b1") && fake.called("serve"),
        "{:?}",
        fake.calls()
    );
}

#[test]
fn a_df_failure_is_said_and_the_pass_goes_on() {
    // Given
    let fake = Fake {
        df_fails: Some("df did not answer within 30s".into()),
        ..short_disk(1)
    };

    // When
    let log = run_pass(&fake, MergeOn::Gate, &mut State::default(), false);

    // Then
    assert!(
        log.iter()
            .any(|l| l == "#1 is not gated: df failed: df did not answer within 30s"),
        "{log:?}"
    );
    assert!(fake.called("merge 2 h2"), "{:?}", fake.calls());
}

#[test]
fn a_du_failure_is_said_and_du_is_tried_again_next_pass() {
    // Given: du fails on the first pass only
    let fake = short_disk(2);
    *fake.du_fails_once.borrow_mut() = Some("du did not answer within 120s".into());
    let mut state = State::default();

    // When
    let first = run_pass(&fake, MergeOn::Gate, &mut state, false);
    let second = run_pass(&fake, MergeOn::Gate, &mut state, false);

    // Then
    assert!(fake.called("merge 2 h2"), "{:?}", fake.calls());
    assert!(
        first.iter().any(|l| l == "#1 is not gated: the disk has under 60 GB free (10 GB); du failed: du did not answer within 120s; it is tried again next pass"),
        "{first:?}"
    );
    assert!(
        second
            .iter()
            .any(|l| l
                .starts_with("the disk has 10 GB free; the largest targets: 62 GB /t/w-desktop;")),
        "{second:?}"
    );
}

#[test]
fn a_du_that_keeps_failing_is_said_once() {
    // Given: du fails on two passes running
    let fake = short_disk(3);
    let mut state = State::default();
    let mut pass = || {
        *fake.du_fails_once.borrow_mut() = Some("du did not answer within 120s".into());
        run_pass(&fake, MergeOn::Gate, &mut state, false)
    };

    // When
    let first = pass();
    let second = pass();

    // Then
    let said = |log: &[String]| log.iter().filter(|l| l.contains("du failed")).count();
    assert_eq!(
        (said(&first), said(&second)),
        (1, 0),
        "{first:?} {second:?}"
    );
}
