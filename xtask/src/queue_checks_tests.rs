use super::*;

fn run(status: &str, conclusion: &str) -> String {
    format!(
        r#"{{"__typename":"CheckRun","name":"ci","status":"{status}","conclusion":"{conclusion}"}}"#
    )
}

fn rollup(items: &[String]) -> String {
    format!(r#"{{"statusCheckRollup":[{}]}}"#, items.join(","))
}

#[test]
fn completed_successful_or_skipped_checks_pass() {
    let json = rollup(&[
        run("COMPLETED", "SUCCESS"),
        run("COMPLETED", "SKIPPED"),
        run("COMPLETED", "NEUTRAL"),
    ]);
    assert_eq!(checks_gate(&json), Ok(Gate::Passed));
}

#[test]
fn a_check_still_running_is_no_gate_yet() {
    let json = rollup(&[run("COMPLETED", "SUCCESS"), run("IN_PROGRESS", "")]);
    assert_eq!(checks_gate(&json), Ok(Gate::None));
    let status =
        rollup(&[r#"{"__typename":"StatusContext","context":"x","state":"PENDING"}"#.to_owned()]);
    assert_eq!(checks_gate(&status), Ok(Gate::None));
}

#[test]
fn a_failed_cancelled_or_timed_out_check_fails() {
    for conclusion in [
        "FAILURE",
        "CANCELLED",
        "TIMED_OUT",
        "ACTION_REQUIRED",
        "STARTUP_FAILURE",
    ] {
        let json = rollup(&[run("COMPLETED", "SUCCESS"), run("COMPLETED", conclusion)]);
        assert_eq!(checks_gate(&json), Ok(Gate::Failed), "{conclusion}");
    }
    let status =
        rollup(&[r#"{"__typename":"StatusContext","context":"x","state":"ERROR"}"#.to_owned()]);
    assert_eq!(checks_gate(&status), Ok(Gate::Failed));
}

#[test]
fn no_checks_or_an_unknown_conclusion_is_refused() {
    assert!(
        checks_gate(&rollup(&[]))
            .expect_err("none")
            .contains("no checks")
    );
    let odd = rollup(&[run("COMPLETED", "SOMETHING_NEW")]);
    assert!(
        checks_gate(&odd)
            .expect_err("odd")
            .contains("SOMETHING_NEW")
    );
}
