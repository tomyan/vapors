//! A job's `if:`: `${{ !cancelled() }}`, which runs a job after an
//! earlier one it needs failed, is planned (the gate stops at its first
//! failure, so it reaches the job only when every earlier one passed);
//! any other condition is refused, naming the job.

use super::refusal_tests::refused;
use super::tests::{WORKFLOW, plan_of};

const GPU: &str = "    timeout-minutes: 20\n";

#[test]
fn a_job_that_runs_unless_cancelled_is_planned() {
    // Given
    let workflow = WORKFLOW.replace(
        GPU,
        &format!("{GPU}    needs: ci\n    if: ${{{{ !cancelled() }}}}\n"),
    );

    // Then
    assert!(plan_of(&workflow).is_ok(), "{:?}", plan_of(&workflow).err());
}

#[test]
fn a_job_if_the_gate_cannot_evaluate_is_refused_naming_the_job() {
    for condition in [
        "github.event_name == 'push'",
        "${{ always() }}",
        "failure()",
    ] {
        let workflow = WORKFLOW.replace(GPU, &format!("{GPU}    if: {condition}\n"));
        refused(&workflow, "gpu: `if:");
    }
}
