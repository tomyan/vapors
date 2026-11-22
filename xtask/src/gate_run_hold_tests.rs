//! Each step's hold before it runs: its wait not counted against the
//! job, and a hold that cannot be had failing the gate.

use std::path::Path;
use std::time::Duration;

use super::*;

fn planned(name: &str, job_timeout: Duration) -> PlannedStep {
    PlannedStep {
        name: name.to_owned(),
        run: "true".to_owned(),
        env: std::collections::BTreeMap::new(),
        dir: ".".to_owned(),
        job: "web".to_owned(),
        job_timeout,
        step_timeout: None,
    }
}

/// `steps` run by the gate with `hold` before each, in a scratch run dir.
fn gate(steps: &[PlannedStep], hold: HoldFor<'_>) -> ExitCode {
    let base = crate::scratch::dir("gate-hold-run");
    let output = base.join("out");
    let reporter = Reporter::new(false);
    crate::run_tmp::around("gate", &base, |tmp| {
        run_in(steps, (&output, Wrap::SwitchedOff), &reporter, tmp, hold)
    })
    .expect("no leak")
}

#[test]
fn a_wait_for_a_hold_inside_a_job_is_not_counted_against_the_job_s_timeout() {
    // Given: a job of 1 s whose second step waits 1.5 s for its hold;
    // the wait is the thing measured here, so it is a real one
    let job = Duration::from_secs(1);
    let steps = [
        planned("web: trunk", job),
        planned("web: page contract", job),
    ];
    let mut hold = |step: &PlannedStep| {
        if step.name == "web: page contract" {
            std::thread::sleep(Duration::from_millis(1500));
        }
        Ok(())
    };

    // When
    let code = gate(&steps, &mut hold);

    // Then
    assert_eq!(code, ExitCode::SUCCESS, "the wait used up the job");
}

#[test]
fn a_hold_that_cannot_be_had_fails_the_gate_before_its_step_runs() {
    // Given
    let marker = crate::scratch::dir("gate-hold-refused");
    let ran = marker.join("ran");
    let mut step = planned("web: page contract", Duration::from_mins(1));
    step.run = format!("touch {}", ran.display());
    let mut hold = |_: &PlannedStep| Err("the gate lock is still held".to_owned());

    // When
    let code = gate(&[step], &mut hold);

    // Then
    assert_eq!(code, ExitCode::FAILURE);
    assert!(!Path::new(&ran).exists(), "the step ran without its hold");
}
