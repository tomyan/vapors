//! Running one step and capturing what it produced.

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::steps::Step;

/// What happened when a step ran.
pub struct Outcome {
    /// True when the process exited with status zero.
    pub passed: bool,
    /// Combined stdout and stderr, in the order each stream was read.
    pub output: String,
    /// Wall-clock time the step took.
    pub duration: Duration,
}

/// Run a step from the current directory and capture its output.
///
/// A step that cannot be started (program missing) is reported as a
/// failure whose output names the program, never as a panic.
pub fn run(step: &Step) -> Outcome {
    let started = Instant::now();
    let result = Command::new(step.program)
        .args(step.args)
        .stdin(Stdio::null())
        .output();
    let duration = started.elapsed();
    match result {
        Ok(output) => Outcome {
            passed: output.status.success(),
            output: combine(&output.stdout, &output.stderr),
            duration,
        },
        Err(error) => Outcome {
            passed: false,
            output: format!("could not start `{}`: {error}", step.program),
            duration,
        },
    }
}

fn combine(stdout: &[u8], stderr: &[u8]) -> String {
    let mut text = String::from_utf8_lossy(stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(stderr));
    text
}
