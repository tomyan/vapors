//! A pass driven through the recording fake (`queue_fake.rs`).

use std::collections::{BTreeMap, VecDeque};
use std::path::PathBuf;

use super::*;
use crate::gate_pr::{Ended, comment};
use crate::queue_config::{MergeOn, QueueConfig};
use crate::queue_fake::{Fake, clear, listed};

pub(super) fn config(merge_on: MergeOn) -> QueueConfig {
    QueueConfig {
        repo: "tomyan/touche".into(),
        merge_on,
        run: PathBuf::from("/q/run"),
        log: std::env::temp_dir().join(format!(
            "xtask-queue-pass-{}-{:?}.log",
            std::process::id(),
            std::thread::current().id()
        )),
        serve: None,
    }
}

/// A clear signed `role` and a passing gate for `head`.
pub(super) fn ready(number: u64, head: &str, role: &str) -> Vec<String> {
    vec![
        clear(role),
        comment(head, number, Ended::Code(0), &format!("gate: pass {head}")),
    ]
}

pub(super) fn run_pass(
    fake: &Fake,
    merge_on: MergeOn,
    state: &mut State,
    dry_run: bool,
) -> Vec<String> {
    let config = config(merge_on);
    let log = Log {
        path: config.log.clone(),
    };
    pass(fake, &config, &log, state, dry_run).expect("passed");
    let text = std::fs::read_to_string(&config.log).unwrap_or_default();
    let _ = std::fs::remove_file(&config.log);
    text.lines()
        .map(|l| l.split_once(' ').map_or(l, |(_, rest)| rest).to_owned())
        .collect()
}

pub(super) fn two_ready() -> Fake {
    let comments = BTreeMap::from([(1, ready(1, "h1", "lead")), (2, ready(2, "h2", "lead"))]);
    Fake {
        prs: vec![listed(1), listed(2)],
        comments: comments.into(),
        ..Fake::default()
    }
}

#[test]
fn a_ready_pr_is_merged_its_stack_retargeted_and_the_pass_ends() {
    let mut stacked = listed(3);
    stacked.base = "b1".into();
    let fake = Fake {
        prs: vec![listed(1), listed(2), stacked],
        ..two_ready()
    };
    let log = run_pass(&fake, MergeOn::Gate, &mut State::default(), false);
    assert!(
        fake.called("merge 1 h1")
            && fake.called("retarget 3")
            && fake.called("delete b1")
            && fake.called("serve")
    );
    assert!(!fake.called("merge 2"), "{:?}", fake.calls());
    assert!(log.iter().any(|l| l == "#1 merged at h1"), "{log:?}");
}

#[test]
fn a_skip_or_a_failed_merge_moves_on_to_the_next_pr() {
    let no_clear = two_ready();
    no_clear.comments.borrow_mut().insert(1, Vec::new());
    let log = run_pass(&no_clear, MergeOn::Gate, &mut State::default(), false);
    assert!(
        no_clear.called("merge 2 h2") && log.iter().any(|l| l == "#1 has no review clear"),
        "{log:?}"
    );
    let refused = Fake {
        merge_fails: vec![1],
        ..two_ready()
    };
    let log = run_pass(&refused, MergeOn::Gate, &mut State::default(), false);
    assert!(refused.called("merge 2 h2"), "{:?}", refused.calls());
    assert!(
        log.iter().any(|l| l.starts_with("FAILED #1: gh refused")),
        "{log:?}"
    );
}

#[test]
fn a_head_that_still_differs_after_its_recheck_is_not_acted_on() {
    let fake = two_ready();
    fake.heads
        .borrow_mut()
        .insert(1, VecDeque::from(["hX".to_owned(), "hX".to_owned()]));
    let log = run_pass(&fake, MergeOn::Gate, &mut State::default(), false);
    assert!(
        fake.called("wait 10s") && !fake.called("merge 1"),
        "{:?}",
        fake.calls()
    );
    assert!(log.iter().any(|l| l.contains("still differs")), "{log:?}");
    let lagging = two_ready();
    lagging
        .heads
        .borrow_mut()
        .insert(1, VecDeque::from(["hX".to_owned()]));
    run_pass(&lagging, MergeOn::Gate, &mut State::default(), false);
    assert!(lagging.called("merge 1 h1"), "{:?}", lagging.calls());
}

#[test]
fn main_moving_before_the_merge_stops_it() {
    let fake = two_ready();
    fake.contains
        .borrow_mut()
        .insert("h1".into(), VecDeque::from([true, false]));
    let log = run_pass(&fake, MergeOn::Gate, &mut State::default(), false);
    assert!(!fake.called("merge 1"), "{:?}", fake.calls());
    assert!(log.iter().any(|l| l.contains("main moved")), "{log:?}");
}

#[test]
fn what_github_already_did_after_a_merge_is_said_not_redone() {
    // Given: GitHub retargeted the stacked PR and deleted the merged branch itself
    let mut stacked = listed(3);
    stacked.base = "b1".into();
    let fake = Fake {
        prs: vec![listed(1), stacked],
        github_tidied: true,
        ..two_ready()
    };

    // When
    let log = run_pass(&fake, MergeOn::Gate, &mut State::default(), false);

    // Then
    assert!(
        log.iter().any(|l| l == "#3 is already based on main"),
        "{log:?}"
    );
    assert!(
        log.iter().any(|l| l == "branch b1 is already deleted"),
        "{log:?}"
    );
}
