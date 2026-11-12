use super::*;
use crate::queue_decide::Gate;

const HEAD: &str = "b26d5aa3cf0d40373d621158d8231e3555598eeb";

fn gate_comment(head: &str, code: i32, last: &str) -> String {
    format!(
        "Gate output for {head} (`cargo xtask gate --pr 71`, exit {code}):\n\n```\nok   ci\n{last}\n```\n"
    )
}

#[test]
fn a_comment_ending_queue_hold_holds_the_pr() {
    let comments = [
        "Looks good.".to_owned(),
        "Wait for #70.\n\nqueue: hold\n".to_owned(),
    ];
    assert!(held(&comments));
    assert!(!held(&["queue: hold is how you stop it".to_owned()]));
}

#[test]
fn the_newest_review_clear_names_its_sha() {
    let comments = [
        "review: clear 1111111111111111111111111111111111111111 (lead)".to_owned(),
        "Findings fixed.\n\nreview: clear 2222222222222222222222222222222222222222 (foreman)\n"
            .to_owned(),
    ];
    assert_eq!(
        cleared(&comments),
        Some(Cleared {
            sha: "2222222222222222222222222222222222222222".into(),
            by_lead: false
        })
    );
    assert_eq!(cleared(&["## Review of 2222222".to_owned()]), None);
}

#[test]
fn a_clear_without_its_reviewer_is_not_a_clear() {
    assert_eq!(cleared(&["review: clear 2222222".to_owned()]), None);
    assert_eq!(cleared(&["review: clear 2222222 lead".to_owned()]), None);
    assert_eq!(cleared(&["review: clear 2222222 ()".to_owned()]), None);
}

#[test]
fn only_the_lead_or_foreman_clears_a_pr_with_a_full_sha() {
    // A convention, not a security boundary: every comment comes from
    // the same GitHub account.
    const SHA: &str = "2222222222222222222222222222222222222222";
    assert_eq!(
        cleared(&[format!("review: clear {SHA} (impl-budgets)")]),
        None
    );
    let lead = cleared(&[format!("review: clear {SHA} (lead)")]);
    assert!(lead.is_some_and(|c| c.by_lead && c.sha == SHA));
    assert_eq!(
        cleared(&["review: clear 2222222 (lead)".to_owned()]),
        None,
        "a short sha"
    );
    assert_eq!(
        cleared(&["review: clear main (lead)".to_owned()]),
        None,
        "not a sha"
    );
}

#[test]
fn the_newest_gate_comment_for_the_head_decides_its_gate() {
    let passed = gate_comment(HEAD, 0, &format!("gate: pass {HEAD}"));
    let failed = gate_comment(HEAD, 1, "FAIL   3/24 steps");
    let other_head = gate_comment(
        "0000000000000000000000000000000000000000",
        0,
        "gate: pass 0000000000000000000000000000000000000000",
    );
    assert_eq!(gate(std::slice::from_ref(&passed), HEAD), Gate::Passed);
    assert_eq!(gate(&[passed.clone(), failed.clone()], HEAD), Gate::Failed);
    assert_eq!(gate(&[failed, passed], HEAD), Gate::Passed);
    assert_eq!(gate(&[other_head], HEAD), Gate::None);
}

#[test]
fn a_gate_killed_by_a_signal_at_the_head_failed() {
    let killed = crate::gate_pr::comment(HEAD, 71, crate::gate_pr::Ended::Signal(9), "ok   ci");
    assert_eq!(gate(&[killed], HEAD), Gate::Failed);
}
