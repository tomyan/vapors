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
        fork: false,
        protected: None,
        cleared_by_lead: false,
        backed_off: false,
        checks: false,
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
        clear: Clear::Changed("abc is not a patch the clear covers".into()),
        ..ready()
    };
    assert_eq!(
        decide(&none),
        Decision::Skip("#71 has no review clear".into())
    );
    assert_eq!(
        decide(&changed),
        Decision::Skip(
            "#71 changed since review clear: abc is not a patch the clear covers".into()
        )
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

#[test]
fn a_pr_from_a_fork_is_skipped() {
    let fork = Facts {
        fork: true,
        ..ready()
    };
    assert_eq!(
        decide(&fork),
        Decision::Skip("#71 is from a fork; the queue pushes only to this repository".into())
    );
}

#[test]
fn a_pr_touching_the_factory_merges_only_with_the_lead_s_clear() {
    let factory = Facts {
        protected: Some("xtask/src/queue.rs".into()),
        ..ready()
    };
    assert_eq!(
        decide(&factory),
        Decision::Skip(
            "#71 touches xtask/src/queue.rs, so it needs a clear signed (lead), not (foreman)"
                .into()
        )
    );
    let led = Facts {
        cleared_by_lead: true,
        ..factory
    };
    assert_eq!(decide(&led), Decision::Merge);
}

#[test]
fn running_checks_are_waited_for_and_a_backed_off_gate_is_not_rerun() {
    let running = Facts {
        gate: Gate::None,
        checks: true,
        ..ready()
    };
    assert_eq!(
        decide(&running),
        Decision::Skip("#71's checks are running".into())
    );
    let backed = Facts {
        gate: Gate::None,
        backed_off: true,
        ..ready()
    };
    assert_eq!(
        decide(&backed),
        Decision::Skip(
            "#71's last gate at aaa posted nothing; it is retried after its backoff".into()
        )
    );
}

#[test]
fn a_checks_pr_behind_main_that_rebases_cleanly_is_updated() {
    let behind = Facts {
        contains_main: false,
        gate: Gate::None,
        checks: true,
        ..ready()
    };
    assert_eq!(decide(&behind), Decision::Regate);
}
