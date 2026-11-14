//! Whose hold a run may go on under, and the variable that says so.

use std::time::Duration;

use super::*;

fn holder(pid: u32, worktree: &str, form: Form) -> Holder {
    let mut holder = Holder::me(worktree, Duration::from_secs(60), form, "x");
    holder.owner.pid = pid;
    holder
}

#[test]
fn the_variable_is_a_pid_or_refused_naming_it() {
    assert_eq!(holder_from_env(None), Ok(None));
    assert_eq!(holder_from_env(Some("42")), Ok(Some(42)));
    for text in ["", "abc", "-1"] {
        let refusal = holder_from_env(Some(text)).expect_err(text);
        assert!(
            refusal.contains(HOLDER_ENV) && refusal.contains(&format!("`{text}`")),
            "{refusal}"
        );
    }
}

#[test]
fn a_run_goes_on_under_its_parent_quiet_and_a_hold_from_its_own_worktree() {
    assert!(is_mine(&holder(42, "/w/a", Form::Run), Some(42), "/w/b"));
    assert!(is_mine(&holder(42, "/w/a", Form::Hold), None, "/w/a"));
}

#[test]
fn a_run_waits_on_another_worktree_s_hold_and_on_a_gate_in_its_own() {
    assert!(!is_mine(&holder(42, "/w/a", Form::Hold), None, "/w/b"));
    assert!(!is_mine(&holder(42, "/w/a", Form::Gate), None, "/w/a"));
    assert!(!is_mine(&holder(42, "/w/a", Form::Run), Some(43), "/w/a"));
}

#[test]
fn a_gate_goes_on_under_its_own_worktree_s_hold_or_its_parent_quiet() {
    assert!(!gate_takes_a_hold(
        Some(&holder(42, "/w/a", Form::Hold)),
        None,
        "/w/a"
    ));
    assert!(!gate_takes_a_hold(
        Some(&holder(42, "/w/a", Form::Run)),
        Some(42),
        "/w/b"
    ));
}

#[test]
fn a_gate_takes_a_hold_when_none_is_held_or_another_s_is() {
    assert!(gate_takes_a_hold(None, None, "/w/a"));
    assert!(gate_takes_a_hold(
        Some(&holder(42, "/w/b", Form::Hold)),
        None,
        "/w/a"
    ));
}
