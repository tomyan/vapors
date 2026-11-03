//! Starting the sccache server, outside any run's TMPDIR.
//!
//! `sccache --show-stats` does not start a server; the first rustc that
//! sccache wraps does, and the server keeps that process's environment.
//! On a machine with no server running (every GitHub runner), that was a
//! step of `cargo xtask lint` under its run's TMPDIR (`run_tmp.rs`),
//! removed when the run ended; the next run's compiles then failed with
//! exit status 254 (vapors #117, run 41533040678; reproduced here with
//! a cold server of its own). So xtask starts the server itself first,
//! with TMPDIR unset: the system's default, which outlives every run.
//! Declared limit: a server already running is used as it is, whatever
//! TMPDIR it was started with.

use crate::command::Outcome;
use crate::steps::Step;

/// What sccache says when a server is already listening.
const RUNNING: &str = "Address in use";

/// Start the server, unless one is running.
///
/// # Errors
/// sccache could not start one, naming why and the fix.
pub fn start() -> Result<(), String> {
    judged(&crate::command::run(&start_step()))
}

/// `sccache --start-server` with no TMPDIR and the server's settings.
#[must_use]
pub fn start_step() -> Step {
    let mut step = Step::new("sccache", "sccache", &["--start-server"], super::FIX);
    step.env = super::SERVER_ENV
        .iter()
        .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
        .collect();
    step.tmpdir = crate::step::TmpDir::Unset;
    step.deadline = Some(super::ANSWER_DEADLINE);
    step
}

/// Started, or already running: ready. Anything else: refused.
fn judged(outcome: &Outcome) -> Result<(), String> {
    if outcome.passed || outcome.output.contains(RUNNING) {
        return Ok(());
    }
    Err(format!(
        "sccache could not start its server: {}; {}",
        outcome.output.trim(),
        super::FIX
    ))
}

#[cfg(test)]
#[path = "sccache_start_tests.rs"]
mod tests;
