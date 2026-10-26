//! What the merge queue does to a PR it decided on (`queue_pass.rs`), all
//! through `Ops`. A merge: squash at the judged head, point the PRs stacked
//! on its branch at main, delete the branch once none is based on it, and
//! re-publish `/`. A re-gate: in checks mode, ask GitHub to update the
//! branch; in gate mode, rebase onto main when behind, push with a lease
//! on the judged head, wait for GitHub to report the new head, and run
//! `gate --pr`; a gate that posts nothing backs the head off. Every step
//! is logged.

use std::time::Duration;

use crate::queue_comments::gate;
use crate::queue_config::{MergeOn, QueueConfig};
use crate::queue_decide::Gate;
use crate::queue_list::Listed;
use crate::queue_log::Log;
use crate::queue_ops::{Gated, Ops};
use crate::queue_state::State;

/// How long GitHub may take to report a pushed head. Seen 2026-10-26:
/// `gh pr view` gave the old head for a few seconds after a push.
const HEAD_LAG: Duration = Duration::from_secs(60);

/// How often the pushed head is read again while waiting for it.
const HEAD_POLL: Duration = Duration::from_secs(3);

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
pub fn merge(ops: &impl Ops, log: &Log, pr: &Listed, prs: &[Listed]) -> Result<(), String> {
    ops.merge(pr.number, &pr.head)?;
    log.line(&format!("#{} merged at {}", pr.number, pr.head))?;
    if retarget_stack(ops, log, prs, &pr.branch) {
        log.line(&outcome(
            &format!("deleted branch {}", pr.branch),
            ops.delete_branch(&pr.branch),
        ))?;
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
        let _ = log.line(&outcome(&format!("#{number} retargeted to main"), done));
        ok
    })
}

fn outcome(what: &str, done: Result<(), String>) -> String {
    match done {
        Ok(()) => what.to_owned(),
        Err(why) => format!("FAILED {what}: {why}"),
    }
}

/// Rebase `pr` onto main if `behind`, push it, and gate it (or, for
/// checks, update its branch).
///
/// # Errors
///
/// A step's refusal (the rebase, the lease, the push, the gate).
pub fn regate(
    ops: &impl Ops,
    config: &QueueConfig,
    log: &Log,
    state: &mut State,
    pr: &Listed,
    behind: bool,
) -> Result<(), String> {
    if config.merge_on == MergeOn::Checks {
        return update(ops, log, pr.number);
    }
    let head = if behind {
        rebased(ops, log, pr)?
    } else {
        pr.head.clone()
    };
    let gated = gate_or_back_off(ops, state, pr.number, &head)?;
    log.line(&format!(
        "#{} gated at {head}: {}",
        pr.number,
        gated.ended.words()
    ))?;
    back_off_if_silent(ops, log, state, (pr.number, &head), &gated.tail)
}

/// A gate that posted nothing for `head` backs it off for 30 min, so it
/// does not take the gate slot every pass.
/// Checks mode: GitHub merges main into PR `number`, and its checks rerun.
fn update(ops: &impl Ops, log: &Log, number: u64) -> Result<(), String> {
    ops.update_branch(number)?;
    log.line(&format!("#{number} updated from main; its checks rerun"))
}

/// Gate `head`; a gate that errs (killed at its deadline, or never
/// started) backs `head` off as one that posts nothing does.
fn gate_or_back_off(
    ops: &impl Ops,
    state: &mut State,
    number: u64,
    head: &str,
) -> Result<Gated, String> {
    ops.gate(number, head).map_err(|why| {
        state.back_off(head, ops.now_secs());
        format!("{why}; {head} backed off for 30 min")
    })
}

/// Back off a head whose gate posted nothing, logging the gate's last
/// lines, which say why.
fn back_off_if_silent(
    ops: &impl Ops,
    log: &Log,
    state: &mut State,
    (number, head): (u64, &str),
    tail: &str,
) -> Result<(), String> {
    if gate(&ops.comments(number)?, head) != Gate::None {
        return Ok(());
    }
    state.back_off(head, ops.now_secs());
    let said = format!("#{number}'s gate posted nothing at {head}; backed off for 30 min");
    if tail.is_empty() {
        return log.line(&said);
    }
    log.line(&format!("{said}; its last lines:"))?;
    tail.lines()
        .try_for_each(|line| log.line(&format!("  {line}")))
}

fn rebased(ops: &impl Ops, log: &Log, pr: &Listed) -> Result<String, String> {
    if pr.branch == "main" {
        return Err("the queue never pushes main".into());
    }
    let new = ops
        .rebased(&pr.head)?
        .ok_or_else(|| format!("#{} conflicts with main on rebase", pr.number))?;
    ops.push(&pr.branch, &pr.head, &new)?;
    log.line(&format!(
        "#{} rebased onto main and pushed: {} -> {new}",
        pr.number, pr.head
    ))?;
    wait_for_head(ops, pr.number, &new)?;
    Ok(new)
}

/// Until GitHub reports `head` as PR `number`'s head, within `HEAD_LAG`.
fn wait_for_head(ops: &impl Ops, number: u64, head: &str) -> Result<(), String> {
    for _ in 0..HEAD_LAG.as_secs() / HEAD_POLL.as_secs() {
        if ops.now(number)?.head == head {
            return Ok(());
        }
        ops.wait(HEAD_POLL);
    }
    Err(format!(
        "GitHub did not report #{number}'s head as {head} within {HEAD_LAG:?}"
    ))
}
