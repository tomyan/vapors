//! A merge the queue decided on (`queue_pass.rs`), all through `Ops`:
//! squash at the judged head, see that the PRs stacked on its branch are
//! based on main and that the branch is gone (GitHub does both itself
//! here; the queue does what is left and says which), and re-publish `/`.
//! Every step is logged. A re-gate is `queue_regate.rs`.

use crate::queue_list::Listed;
use crate::queue_log::Log;
use crate::queue_ops::Ops;

/// The numbers of the PRs based on `branch`.
#[must_use]
pub fn stacked_on(prs: &[Listed], branch: &str) -> Vec<u64> {
    prs.iter()
        .filter(|pr| pr.base == branch)
        .map(|pr| pr.number)
        .collect()
}

/// Empty `branch`'s worktree targets now it has merged; a failure is
/// logged, and the merge stands.
fn empty_targets(ops: &impl Ops, log: &Log, branch: &str) -> Result<(), String> {
    match ops.empty_targets(branch) {
        Ok(lines) => lines.iter().try_for_each(|line| log.line(line)),
        Err(why) => log.line(&format!("FAILED to empty {branch}'s targets: {why}")),
    }
}

/// Merge `pr`, empty its targets, retarget its stacked PRs, delete its branch, publish.
///
/// # Errors
///
/// The merge's refusal; a later step's failure is logged.
pub fn merge(ops: &impl Ops, log: &Log, pr: &Listed, prs: &[Listed]) -> Result<(), String> {
    ops.merge(pr.number, &pr.head)?;
    log.line(&format!("#{} merged at {}", pr.number, pr.head))?;
    empty_targets(ops, log, &pr.branch)?;
    if retarget_stack(ops, log, prs, &pr.branch) {
        log.line(&branch_line(&pr.branch, ops.delete_branch(&pr.branch)))?;
    } else {
        log.line(&format!(
            "kept branch {}: a stacked PR could not be retargeted",
            pr.branch
        ))?;
    }
    log.line(&outcome("re-published /", ops.serve()))
}

/// Point every PR based on `branch` at main; whether all were.
fn retarget_stack(ops: &impl Ops, log: &Log, prs: &[Listed], branch: &str) -> bool {
    stacked_on(prs, branch).iter().all(|number| {
        let done = ops.retarget(*number);
        let ok = done.is_ok();
        let line = match done {
            Ok(true) => format!("#{number} retargeted to main"),
            Ok(false) => format!("#{number} is already based on main"),
            Err(why) => format!("FAILED #{number} retargeted to main: {why}"),
        };
        let _ = log.line(&line);
        ok
    })
}

/// The log's line for deleting `branch`.
fn branch_line(branch: &str, done: Result<bool, String>) -> String {
    match done {
        Ok(true) => format!("deleted branch {branch}"),
        Ok(false) => format!("branch {branch} is already deleted"),
        Err(why) => format!("FAILED deleted branch {branch}: {why}"),
    }
}

fn outcome(what: &str, done: Result<(), String>) -> String {
    match done {
        Ok(()) => what.to_owned(),
        Err(why) => format!("FAILED {what}: {why}"),
    }
}
