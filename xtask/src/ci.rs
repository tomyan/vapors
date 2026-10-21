//! Running a list of steps in order and stopping at the first failure.

use std::process::ExitCode;
use std::time::Instant;

use crate::command;
use crate::report::Reporter;
use crate::steps::Step;

/// Run `steps` in order. Stops at the first failure so the slow steps
/// never run when a fast one has already failed.
pub fn run(steps: &[Step], reporter: &Reporter) -> ExitCode {
    let started = Instant::now();
    let mut passed = 0;
    for step in steps {
        let outcome = command::run(step);
        reporter.step(step, &outcome);
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
