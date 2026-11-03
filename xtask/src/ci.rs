//! Running a list of steps in order and stopping at the first failure.

use std::path::Path;
use std::process::ExitCode;
use std::time::Instant;

use crate::command;
use crate::report::Reporter;
use crate::steps::Step;

/// Run `steps` in order, each with this run's own TMPDIR (`run_tmp`).
/// Stops at the first failure so the slow steps never run when a fast
/// one has already failed.
pub fn run(steps: &[Step], reporter: &Reporter) -> ExitCode {
    crate::run_tmp::around_temp_dir("this run", |tmp| run_in(steps, reporter, tmp))
}

fn run_in(steps: &[Step], reporter: &Reporter, tmp: &Path) -> ExitCode {
    let started = Instant::now();
    let mut passed = 0;
    for step in steps {
        let step = crate::run_tmp::with_tmpdir(step.clone(), tmp);
        let outcome = command::run(&step);
        reporter.step(&step, &outcome);
        if !outcome.passed {
            break;
        }
        passed += 1;
    }
    Reporter::summary(passed, steps.len(), started.elapsed());
    if passed == steps.len() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

#[cfg(test)]
#[path = "ci_tests.rs"]
mod tests;
