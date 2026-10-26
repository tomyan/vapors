//! One PR judged for a pass (`queue_pass.rs`): the facts `decide` reads,
//! from its comments, gh and git. The facts that need no git (held, a
//! draft, a fork, its base, its clear) are decided first, so a PR waiting
//! on any of them costs no fetch.

use crate::queue_comments::{Cleared, cleared, gate, held};
use crate::queue_config::{MergeOn, QueueConfig};
use crate::queue_decide::{Clear, Decision, Facts, Gate, decide};
use crate::queue_list::Listed;
use crate::queue_ops::Ops;
use crate::queue_state::State;

/// Paths only the lead may clear: the factory (the queue's own units and
/// script are under xtask/), CI, the factory's settings and pins, and
/// what decides how `cargo xtask` builds and runs (the alias and build
/// settings in .cargo/, the toolchain, cargo-deny's policy). A declared
/// limit: the root Cargo.toml's workspace members can also change what
/// builds, and a change to it merges on a foreman's clear; reviewers
/// read it as a factory change.
pub const PROTECTED: [&str; 6] = [
    "xtask/",
    ".github/",
    ".config/",
    ".cargo/",
    "rust-toolchain.toml",
    "deny.toml",
];

/// A PR's decision, and whether it is behind main.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Judged {
    /// What the pass does with it.
    pub decision: Decision,
    /// Whether its head lacks origin/main.
    pub behind: bool,
}

/// Judge `pr`.
///
/// # Errors
///
/// gh or git refusing; the PR is then logged and skipped, never acted on.
pub fn judge(
    ops: &impl Ops,
    config: &QueueConfig,
    state: &State,
    pr: &Listed,
) -> Result<Judged, String> {
    let comments = ops.comments(pr.number)?;
    let clear = cleared(&comments);
    let mut facts = cheap_facts(config, pr, &comments, clear.as_ref());
    let (Some(clear), Decision::Merge) = (clear, decide(&as_if_ready(&facts))) else {
        let decision = decide(&as_if_ready(&facts));
        return Ok(Judged {
            decision,
            behind: false,
        });
    };
    git_facts(ops, config, state, pr, &comments, &clear, &mut facts)?;
    Ok(Judged {
        decision: decide(&facts),
        behind: !facts.contains_main,
    })
}

/// The facts git, gh and the state give, for a PR the cheap facts let
/// through.
fn git_facts(
    ops: &impl Ops,
    config: &QueueConfig,
    state: &State,
    pr: &Listed,
    comments: &[String],
    clear: &Cleared,
    facts: &mut Facts,
) -> Result<(), String> {
    ops.fetch(&[&pr.head, &clear.sha])?;
    facts.clear = ops.clear_holds(&pr.head, &clear.sha)?;
    facts.protected = protected(&ops.touched(&pr.head)?);
    facts.contains_main = ops.contains_main(&pr.head)?;
    facts.gate = match config.merge_on {
        MergeOn::Gate => gate(comments, &pr.head),
        MergeOn::Checks => ops.checks(pr.number)?,
    };
    facts.backed_off = state.backed_off(&pr.head, ops.now_secs());
    if !facts.contains_main {
        facts.rebase_clean = ops.rebased(&pr.head)?.is_some();
    }
    Ok(())
}

/// The facts from the listing and the comments alone.
fn cheap_facts(
    config: &QueueConfig,
    pr: &Listed,
    comments: &[String],
    clear: Option<&Cleared>,
) -> Facts {
    Facts {
        number: pr.number,
        head: pr.head.clone(),
        base_is_main: pr.base == "main",
        draft: pr.draft,
        held: held(comments),
        clear: clear.map_or(Clear::Missing, |_| Clear::Holds),
        contains_main: true,
        gate: Gate::None,
        rebase_clean: true,
        fork: pr.fork,
        protected: None,
        cleared_by_lead: clear.is_some_and(|c| c.by_lead),
        backed_off: false,
        checks: config.merge_on == MergeOn::Checks,
    }
}

/// The cheap facts with everything git would say set to ready, so only
/// what they say can make the PR wait.
fn as_if_ready(facts: &Facts) -> Facts {
    Facts {
        gate: Gate::Passed,
        ..facts.clone()
    }
}

/// The first changed path only the lead may clear.
fn protected(touched: &[String]) -> Option<String> {
    touched
        .iter()
        .find(|path| {
            PROTECTED
                .iter()
                .any(|p| path.starts_with(p) || path.as_str() == p.trim_end_matches('/'))
        })
        .cloned()
}

#[cfg(test)]
#[path = "queue_judge_tests.rs"]
mod tests;
