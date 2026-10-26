//! The merge queue's calls that leave the machine: gh (listing open PRs
//! with their comments, merging, retargeting, deleting a branch) and git
//! fetch and push. Each runs within its deadline, and its refusal is
//! returned as text for the queue's log.

use std::path::Path;
use std::process::Command;
use std::time::Duration;

use crate::deadline::{Finished, run_with_deadline};
use crate::gate_pr_gh::GH_DEADLINE;

/// git fetch and push's deadline. Measured 2026-10-26 on the dev box: a
/// fetch of main took 769 to 899 ms over four, and a `--no-verify` push
/// of a branch to GitHub 1.6 to 1.8 s over three; 60 s is 33 times the
/// slowest push.
pub const GIT_REMOTE_DEADLINE: Duration = Duration::from_secs(60);

/// Squash-merge PR `number` of `repo`, only while its head is `head`.
///
/// # Errors
///
/// gh's refusal (a head that moved, a conflict), as text.
pub fn merge(repo: &str, number: u64, head: &str) -> Result<(), String> {
    gh(&[
        "pr",
        "merge",
        &number.to_string(),
        "--repo",
        repo,
        "--squash",
        "--match-head-commit",
        head,
    ])
    .map(|_| ())
}

/// Point PR `number` at main (the REST API: gh's GraphQL edit fails on
/// the projects-classic deprecation).
///
/// # Errors
///
/// gh's refusal.
pub fn retarget(repo: &str, number: u64) -> Result<bool, String> {
    let pull = format!("repos/{repo}/pulls/{number}");
    if gh(&["api", &pull, "--jq", ".base.ref"])?.trim() == "main" {
        return Ok(false);
    }
    gh(&["api", "-X", "PATCH", &pull, "-f", "base=main"]).map(|_| true)
}

/// Delete `branch` on GitHub.
///
/// # Errors
///
/// gh's refusal.
pub fn delete_branch(repo: &str, branch: &str) -> Result<bool, String> {
    let reference = format!("repos/{repo}/git/refs/heads/{branch}");
    match gh(&["api", "-X", "DELETE", &reference]) {
        Ok(_) => Ok(true),
        // GitHub's answer for a ref that is gone (HTTP 422).
        Err(why) if why.contains("Reference does not exist") => Ok(false),
        Err(why) => Err(why),
    }
}

/// `git fetch origin <refs>` in `dir`.
///
/// # Errors
///
/// git's refusal, or no answer within the deadline.
pub fn fetch(dir: &Path, refs: &[&str]) -> Result<(), String> {
    let mut args = vec!["fetch", "-q", "origin"];
    args.extend_from_slice(refs);
    git_remote(dir, &args)
}

/// Push `new` to `branch`, only if GitHub still has it at `known`. The
/// pre-push hook is skipped (`--no-verify`): it runs lint and the unit
/// tier, which the gate after the push runs anyway. The one thing it
/// also guarded, a push to main, is refused here before anything runs.
///
/// # Errors
///
/// `branch` is main; git's refusal (the lease failed), or no answer.
pub fn push_lease(dir: &Path, branch: &str, known: &str, new: &str) -> Result<(), String> {
    if branch == "main" {
        return Err("the queue never pushes main; refused before pushing".into());
    }
    let args = push_args(branch, known, new);
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    git_remote(dir, &args)
}

/// The push's arguments: no hook, a lease on `known`, `new` to `branch`.
#[must_use]
pub fn push_args(branch: &str, known: &str, new: &str) -> Vec<String> {
    ["push", "-q", "--no-verify"]
        .iter()
        .map(|a| (*a).to_owned())
        .chain([
            format!("--force-with-lease={branch}:{known}"),
            "origin".into(),
            format!("{new}:refs/heads/{branch}"),
        ])
        .collect()
}

/// A gh call within `GH_DEADLINE`; its output when it succeeded.
pub fn gh(args: &[&str]) -> Result<String, String> {
    let mut command = Command::new("gh");
    command.args(args);
    finished(run_with_deadline(command, GH_DEADLINE)?, "gh", GH_DEADLINE)
}

fn git_remote(dir: &Path, args: &[&str]) -> Result<(), String> {
    let mut command = Command::new("git");
    command.arg("-C").arg(dir).args(args);
    for name in ["GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE"] {
        command.env_remove(name);
    }
    finished(
        run_with_deadline(command, GIT_REMOTE_DEADLINE)?,
        "git",
        GIT_REMOTE_DEADLINE,
    )
    .map(|_| ())
}

/// `command` run within `deadline`; its output when it succeeded.
///
/// # Errors
///
/// It cannot start, fails, or does not finish in time, naming `what`.
pub fn within(command: Command, what: &str, deadline: Duration) -> Result<String, String> {
    finished(run_with_deadline(command, deadline)?, what, deadline)
}

fn finished(finished: Finished, what: &str, deadline: Duration) -> Result<String, String> {
    match finished {
        Finished::Exited {
            success: true,
            output,
        } => Ok(output),
        Finished::Exited { output, .. } => Err(format!("{what} refused: {}", output.trim())),
        Finished::TimedOut { .. } => Err(format!("{what} did not answer within {deadline:?}")),
    }
}
