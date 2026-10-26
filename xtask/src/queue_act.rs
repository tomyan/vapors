//! What the merge queue does to a PR it decided on (`queue.rs`). It
//! merges the PR at its head, points the PRs stacked on its branch at
//! main, deletes the branch once none is based on it, and re-publishes
//! `/`. Or it re-gates: it rebases the PR onto main in the queue's own
//! worktree, pushes with a lease on the known head, waits for GitHub to
//! report the new head, and runs `gate --pr` there (or, for checks,
//! asks GitHub to update the branch). Every step is logged.

use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use crate::deadline::{Finished, run_with_deadline};
use crate::queue_config::{MergeOn, QueueConfig};
use crate::queue_log::Log;
use crate::queue_remote::{self, Listed};

/// How long GitHub may take to report a pushed head. Seen 2026-10-26:
/// `gh pr view` gave the old head for a few seconds after a push.
const HEAD_LAG: Duration = Duration::from_secs(60);

/// How long `serve --replace` may take: a release build of the page and
/// its publish. Chosen until measured: a release `cargo xtask web` took
/// about 1 to 2 min on the box under load.
const SERVE_DEADLINE: Duration = Duration::from_mins(20);

/// The numbers of the PRs based on `branch`.
#[must_use]
pub fn stacked_on(prs: &[Listed], branch: &str) -> Vec<u64> {
    prs.iter()
        .filter(|pr| pr.base == branch)
        .map(|pr| pr.number)
        .collect()
}

/// Merge `pr`, retarget its stacked PRs, delete its branch, publish.
///
/// # Errors
///
/// The merge's refusal; a later step's failure is logged.
pub fn merge(config: &QueueConfig, log: &Log, pr: &Listed, prs: &[Listed]) -> Result<(), String> {
    queue_remote::merge(&config.repo, pr.number, &pr.head)?;
    log.line(&format!("#{} merged at {}", pr.number, pr.head))?;
    let stacked = stacked_on(prs, &pr.branch);
    let retargeted = stacked.iter().all(|n| retarget(config, log, *n));
    if retargeted {
        let deleted = queue_remote::delete_branch(&config.repo, &pr.branch);
        log.line(&outcome(&format!("deleted branch {}", pr.branch), deleted))?;
    } else {
        log.line(&format!(
            "kept branch {}: a stacked PR could not be retargeted",
            pr.branch
        ))?;
    }
    match &config.serve {
        Some(dir) => log.line(&outcome("re-published /", serve(dir))),
        None => Ok(()),
    }
}

fn retarget(config: &QueueConfig, log: &Log, number: u64) -> bool {
    let done = queue_remote::retarget(&config.repo, number);
    let ok = done.is_ok();
    let _ = log.line(&outcome(&format!("#{number} retargeted to main"), done));
    ok
}

fn outcome(what: &str, done: Result<(), String>) -> String {
    match done {
        Ok(()) => what.to_owned(),
        Err(why) => format!("FAILED {what}: {why}"),
    }
}

/// Rebase `pr` onto main if behind, push it, and gate it.
///
/// # Errors
///
/// A step's refusal (the lease, the push, the gate's start).
pub fn regate(config: &QueueConfig, log: &Log, pr: &Listed, behind: bool) -> Result<(), String> {
    if config.merge_on == MergeOn::Checks {
        if !behind {
            return log.line(&format!("#{}'s checks are running", pr.number));
        }
        crate::queue_checks::update_branch(&config.repo, pr.number)?;
        return log.line(&format!(
            "#{} updated from main; its checks rerun",
            pr.number
        ));
    }
    let head = if behind {
        rebased(config, log, pr)?
    } else {
        pr.head.clone()
    };
    let code = gate(&config.run, pr.number, &head)?;
    log.line(&format!("#{} gated at {head}: exit {code}", pr.number))
}

fn rebased(config: &QueueConfig, log: &Log, pr: &Listed) -> Result<String, String> {
    let new = crate::queue_git::rebase_clean(&config.run, "origin/main", &pr.head)?
        .ok_or_else(|| format!("#{} conflicts with main on rebase", pr.number))?;
    queue_remote::push_lease(&config.run, &pr.branch, &pr.head, &new)?;
    log.line(&format!(
        "#{} rebased onto main and pushed: {} -> {new}",
        pr.number, pr.head
    ))?;
    wait_for_head(&config.repo, pr.number, &new)?;
    Ok(new)
}

/// Until GitHub reports `head` as PR `number`'s head, within `HEAD_LAG`.
fn wait_for_head(repo: &str, number: u64, head: &str) -> Result<(), String> {
    let started = Instant::now();
    while started.elapsed() < HEAD_LAG {
        if crate::gate_pr_gh::find_in(repo, number)?.head == head {
            return Ok(());
        }
        std::thread::sleep(Duration::from_secs(3));
    }
    Err(format!(
        "GitHub did not report #{number}'s head as {head} within {HEAD_LAG:?}"
    ))
}

/// `cargo xtask gate --pr N` in the queue's worktree, at `head`, its
/// pinned siblings moved to their pins first.
fn gate(run: &Path, number: u64, head: &str) -> Result<i32, String> {
    crate::queue_git::git(run, &["checkout", "-q", "--detach", head])?;
    crate::queue_pins::move_siblings(run)?;
    let status = Command::new("cargo")
        .args(["xtask", "gate", "--pr", &number.to_string()])
        .current_dir(run)
        .status()
        .map_err(|e| format!("could not start the gate: {e}"))?;
    Ok(status.code().unwrap_or(1))
}

/// Re-publish `/` from `dir`: main, its pinned siblings, `serve --replace`.
fn serve(dir: &Path) -> Result<(), String> {
    queue_remote::fetch(dir, &["main"])?;
    crate::queue_git::git(dir, &["checkout", "-q", "--detach", "origin/main"])?;
    crate::queue_pins::move_siblings(dir)?;
    let mut command = Command::new("cargo");
    command
        .args(["xtask", "serve", "--replace"])
        .current_dir(dir);
    match run_with_deadline(command, SERVE_DEADLINE)? {
        Finished::Exited { success: true, .. } => Ok(()),
        Finished::Exited { output, .. } => Err(format!(
            "serve --replace failed: {}",
            crate::gate_pr::tail(&output, 5)
        )),
        Finished::TimedOut { .. } => Err(format!(
            "serve --replace did not finish within {SERVE_DEADLINE:?}"
        )),
    }
}

#[cfg(test)]
#[path = "queue_act_tests.rs"]
mod tests;
