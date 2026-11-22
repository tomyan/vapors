//! Running the gate's plan: in order, stopping at the first failure,
//! each step killed at its deadline. A job's `timeout-minutes` bounds
//! its steps together, from when its first step starts; a step's own
//! `timeout-minutes` bounds it alone. Every step gets `GITHUB_OUTPUT`,
//! a file under target/, as CI gives one, and the run's TMPDIR, and runs
//! with `/dev/dri` hidden, as on CI (`gate_sandbox.rs`). Each step's
//! line is followed by the machine's load at its start and end.
//!
//! Before each step the gate takes the hold it needs (`gate_hold.rs`).
//! The time spent waiting for it is added back to the job's clock, so a
//! wait never uses up the job's `timeout-minutes`; a hold that cannot be
//! had fails the gate before the step runs.

use std::path::Path;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use crate::command;
use crate::gate_plan::PlannedStep;
use crate::report::Reporter;
use crate::steps::Step;

#[path = "gate_sandbox.rs"]
mod sandbox;
pub use sandbox::{Wrap, wrap as sandbox_wrap};

/// Takes the hold a step needs before it runs.
pub type HoldFor<'a> = &'a mut dyn FnMut(&PlannedStep) -> Result<(), String>;

/// Run `planned`, each step with this run's own TMPDIR (`run_tmp`) and
/// as `wrap` says, after `hold` takes what it needs; `output` is the
/// `GITHUB_OUTPUT` file.
pub fn run(
    planned: &[PlannedStep],
    (output, wrap): (&Path, Wrap),
    reporter: &Reporter,
    hold: HoldFor<'_>,
) -> ExitCode {
    crate::run_tmp::around_temp_dir("gate", |tmp| {
        run_in(planned, (output, wrap), reporter, tmp, hold)
    })
}

fn run_in(
    planned: &[PlannedStep],
    (output, wrap): (&Path, Wrap),
    reporter: &Reporter,
    tmp: &Path,
    hold: HoldFor<'_>,
) -> ExitCode {
    let started = Instant::now();
    let mut job: Option<(&str, Instant)> = None;
    let mut passed = 0;
    for planned_step in planned {
        let Some(job_started) = held_for(planned_step, job, hold) else {
            break;
        };
        job = Some((&planned_step.job, job_started));
        let mut step = crate::run_tmp::with_tmpdir(step_for(planned_step, output, wrap), tmp);
        step.deadline = Some(deadline(planned_step, job_started.elapsed()));
        let load_at_start = crate::load::now();
        let outcome = command::run(&step);
        reporter.step(&step, &outcome);
        println!("{}", crate::load::line(&load_at_start, &crate::load::now()));
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

/// Take `step`'s hold; when its job's clock started, moved on by the
/// wait so the wait is not counted, or now for a job's first step. None
/// when the hold cannot be had, said as the gate's failure.
fn held_for(
    step: &PlannedStep,
    job: Option<(&str, Instant)>,
    hold: HoldFor<'_>,
) -> Option<Instant> {
    let asked = Instant::now();
    if let Err(why) = hold(step) {
        println!(
            "FAIL gate: {}: its hold: {why}; then rerun `cargo xtask gate`",
            step.name
        );
        return None;
    }
    match job {
        Some((name, at)) if name == step.job => Some(at + asked.elapsed()),
        _ => Some(Instant::now()),
    }
}

/// The gate's last lines: an unsandboxed run's notice again, then the
/// pass line, which stays last (`gate_pr` reads it).
#[must_use]
pub fn pass_lines(head: &str, wrap: Wrap) -> Vec<String> {
    let notice = wrap.notice().map(|n| format!("gate: {n}"));
    notice
        .into_iter()
        .chain([format!("gate: pass {head}")])
        .collect()
}

/// The time `planned` may take, `spent` into its job.
#[must_use]
pub fn deadline(planned: &PlannedStep, spent: Duration) -> Duration {
    let left = planned.job_timeout.saturating_sub(spent);
    planned.step_timeout.map_or(left, |own| own.min(left))
}

/// A step running `planned`'s script as CI does.
#[must_use]
pub fn step_for(planned: &PlannedStep, output: &Path, wrap: Wrap) -> Step {
    let script = ["bash", "-eo", "pipefail", "-c", planned.run.as_str()];
    let (program, args) = match wrap {
        Wrap::HideGpuDevice => ("bwrap", [&sandbox::HIDE_GPU_DEVICE[..], &script].concat()),
        Wrap::NoRenderNode | Wrap::SwitchedOff => ("bash", script[1..].to_vec()),
    };
    let mut step = Step::new(
        &planned.name,
        program,
        &args,
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

#[cfg(test)]
#[path = "gate_run_hold_tests.rs"]
mod hold_tests;
