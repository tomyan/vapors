//! Running the gate's plan: in order, stopping at the first failure,
//! each step killed at its deadline. A job's `timeout-minutes` bounds
//! its steps together, from when its first step starts; a step's own
//! `timeout-minutes` bounds it alone. Every step gets `GITHUB_OUTPUT`,
//! a file under target/, as CI gives one.

use std::path::Path;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use crate::command;
use crate::gate_plan::PlannedStep;
use crate::report::Reporter;
use crate::steps::Step;

/// Run `planned`; `output` is the `GITHUB_OUTPUT` file.
pub fn run(planned: &[PlannedStep], output: &Path, reporter: &Reporter) -> ExitCode {
    let started = Instant::now();
    let mut job: Option<(&str, Instant)> = None;
    let mut passed = 0;
    for planned_step in planned {
        let job_started = match job {
            Some((name, at)) if name == planned_step.job => at,
            _ => Instant::now(),
        };
        job = Some((&planned_step.job, job_started));
        let mut step = step_for(planned_step, output);
        step.deadline = Some(deadline(planned_step, job_started.elapsed()));
        let outcome = command::run(&step);
        reporter.step(&step, &outcome);
        if !outcome.passed {
            break;
        }
        passed += 1;
    }
    Reporter::summary(passed, planned.len(), started.elapsed());
    if passed == planned.len() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// The time `planned` may take, `spent` into its job.
#[must_use]
pub fn deadline(planned: &PlannedStep, spent: Duration) -> Duration {
    let left = planned.job_timeout.saturating_sub(spent);
    planned.step_timeout.map_or(left, |own| own.min(left))
}

/// A step running `planned`'s script as CI does.
#[must_use]
pub fn step_for(planned: &PlannedStep, output: &Path) -> Step {
    let mut step = Step::new(
        &planned.name,
        "bash",
        &["-eo", "pipefail", "-c", &planned.run],
        "fix it, then rerun `cargo xtask gate`",
    );
    step.env = planned.env.clone().into_iter().collect();
    step.env
        .push(("GITHUB_OUTPUT".to_owned(), output.display().to_string()));
    if planned.dir != "." {
        step.dir = Some(planned.dir.clone());
    }
    step
}

#[cfg(test)]
#[path = "gate_run_tests.rs"]
mod tests;
