use super::*;

fn ready() -> Facts {
    Facts {
        number: 71,
        head: "aaa".into(),
        base_is_main: true,
        draft: false,
        held: false,
        clear: Clear::Holds,
        contains_main: true,
        gate: Gate::Passed,
        rebase_clean: true,
    }
}

#[test]
fn a_reviewed_gated_pr_on_main_is_merged() {
    assert_eq!(decide(&ready()), Decision::Merge);
}

#[test]
fn a_held_or_draft_pr_is_skipped_saying_so() {
    let held = Facts {
        held: true,
        ..ready()
    };
    let draft = Facts {
        draft: true,
        ..ready()
    };
    assert_eq!(
        decide(&held),
        Decision::Skip("#71 is held (queue: hold)".into())
    );
    assert_eq!(decide(&draft), Decision::Skip("#71 is a draft".into()));
}

#[test]
fn a_pr_on_another_base_waits_for_it() {
    let stacked = Facts {
        base_is_main: false,
        ..ready()
    };
    assert_eq!(
        decide(&stacked),
        Decision::Skip("#71 is based on another branch, not main".into())
    );
}

#[test]
fn a_pr_without_a_holding_review_clear_is_left_for_an_agent() {
    let none = Facts {
        clear: Clear::Missing,
        ..ready()
    };
    let changed = Facts {
        clear: Clear::Changed,
        ..ready()
    };
    assert_eq!(
        decide(&none),
        Decision::Skip("#71 has no review clear".into())
    );
    assert_eq!(
        decide(&changed),
        Decision::Skip("#71 changed since review clear".into())
    );
}

#[test]
fn a_ready_pr_behind_main_that_rebases_cleanly_is_regated_by_the_queue() {
    let behind = Facts {
        contains_main: false,
        gate: Gate::None,
        ..ready()
    };
    assert_eq!(decide(&behind), Decision::Regate);
}

#[test]
fn a_pr_behind_main_whose_rebase_conflicts_is_left_for_an_agent() {
    let conflict = Facts {
        contains_main: false,
        rebase_clean: false,
        ..ready()
    };
    assert_eq!(
        decide(&conflict),
        Decision::Skip("#71 conflicts with main on rebase".into())
    );
}

#[test]
fn a_pr_on_main_with_no_gate_for_its_head_is_gated_and_a_failed_gate_is_left() {
    let ungated = Facts {
        gate: Gate::None,
        ..ready()
    };
    let failed = Facts {
        gate: Gate::Failed,
        ..ready()
    };
    assert_eq!(decide(&ungated), Decision::Regate);
    assert_eq!(
        decide(&failed),
        Decision::Skip("#71's gate failed at aaa".into())
    );
}
