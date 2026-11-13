//! `cargo xtask scaling`: vapors-perf's scaling judge (properties 1 and
//! 6 of docs/plans/perf-finish-line.md, and the accessibility ceilings
//! per row) for every change it knows, in release, on a clean tree.
//! Nightly on the dev machine (scripts/nightly-scaling.sh), not in the
//! gate (Tom, 2026-11-13): GitHub's runners have no GPU.

use std::process::ExitCode;
use std::time::Duration;

use crate::report::Reporter;
use crate::steps::Step;
use crate::{ci, fail, gate, sccache};

/// The release build of vapors-perf took 104 s with the cache warm
/// (2026-11-13); 30 minutes leaves room for a build that misses it.
const BUILD_DEADLINE: Duration = Duration::from_mins(30);

/// Every change took 71 s together at load average 20 (2026-11-13,
/// 17 s and 19 s for Insert and Resize, the slowest); ten minutes is
/// over eight times that.
const JUDGE_DEADLINE: Duration = Duration::from_mins(10);

/// Builds and runs the judge; ends `scaling: pass <sha>` or
/// `scaling: fail <sha>`.
pub fn run(reporter: &Reporter) -> ExitCode {
    let revision = gate::git(".", &["rev-parse", "HEAD"])
        .and_then(|sha| Ok((gate::git(".", &["status", "--porcelain"])?, sha)))
        .and_then(|(changes, sha)| clean_revision(&sha, &changes));
    let revision = match revision {
        Ok(sha) => sha,
        Err(why) => return fail("scaling", &why),
    };
    println!("scaling: revision {revision}");
    // The server starts at this xtask's priority, before a niced step can.
    if let Err(why) = sccache::stats() {
        return fail("sccache", &why);
    }
    let code = ci::run(&steps(), reporter);
    let verdict = if code == ExitCode::SUCCESS {
        "pass"
    } else {
        "fail"
    };
    println!("scaling: {verdict} {revision}");
    code
}

/// `sha` when `changes` (`git status --porcelain`) is empty: the judge
/// passes or fails a commit, never a tree with changes.
pub fn clean_revision(sha: &str, changes: &str) -> Result<String, String> {
    if changes.trim().is_empty() {
        return Ok(sha.to_owned());
    }
    Err(format!(
        "the tree at {sha} has changes, and the judge passes or fails only a commit; \
         fix: commit or stash, then rerun `cargo xtask scaling`"
    ))
}

/// The build, then the judge over every change.
pub fn steps() -> Vec<Step> {
    let mut build = Step::new(
        "build",
        "cargo",
        &["build", "--release", "-p", "vapors-perf"],
        "fix the build error above",
    );
    build.deadline = Some(BUILD_DEADLINE);
    let mut judge = Step::new(
        "judge",
        "target/release/vapors-perf",
        &["60", "10", "--scaling", "all"],
        "each failing change above names its phase and growth per extra row: make that \
         phase's work depend on what changed (docs/plans/perf-finish-line.md, properties 1 \
         and 6), or change a ceiling in crates/perf/src/scaling_judged.rs with its reason",
    );
    judge.deadline = Some(JUDGE_DEADLINE);
    vec![build, judge]
}

#[cfg(test)]
#[path = "scaling_tests.rs"]
mod tests;
