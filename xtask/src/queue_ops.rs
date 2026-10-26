//! Everything the merge queue does outside itself, behind one trait, so
//! a pass (`queue_pass.rs`) can be driven in tests by a recording fake.
//! `queue_real.rs` holds the real effects: gh, git in the queue's own
//! worktree, `gate --pr` run by the queue's own binary, the publish.

use std::time::Duration;

use crate::gate_pr::Ended;
use crate::queue_decide::{Clear, Gate};
use crate::queue_list::Listed;

/// A PR as it is now, read again just before acting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Now {
    /// Its head commit.
    pub head: String,
    /// `OPEN`, `CLOSED` or `MERGED`.
    pub state: String,
    /// Whether it is a draft.
    pub draft: bool,
}

/// The queue's effects.
pub trait Ops {
    /// The open PRs, oldest first (refused at the list's limit).
    fn open_prs(&self) -> Result<Vec<Listed>, String>;
    /// Every comment of PR `number`, oldest first.
    fn comments(&self, number: u64) -> Result<Vec<String>, String>;
    /// PR `number` as it is now.
    fn now(&self, number: u64) -> Result<Now, String>;
    /// Fetch `refs` into the queue's worktree.
    fn fetch(&self, refs: &[&str]) -> Result<(), String>;
    /// Whether `head` contains origin/main.
    fn contains_main(&self, head: &str) -> Result<bool, String>;
    /// Whether `cleared`'s review clear still describes `head`.
    fn clear_holds(&self, head: &str, cleared: &str) -> Result<Clear, String>;
    /// The files `head` changes over origin/main.
    fn touched(&self, head: &str) -> Result<Vec<String>, String>;
    /// `head` rebased onto origin/main: the new head, or none on a conflict.
    fn rebased(&self, head: &str) -> Result<Option<String>, String>;
    /// Push `new` to `branch` with a lease on `known`.
    fn push(&self, branch: &str, known: &str, new: &str) -> Result<(), String>;
    /// Run `gate --pr number` at `head`, by the queue's own binary.
    fn gate(&self, number: u64, head: &str) -> Result<Ended, String>;
    /// PR `number`'s checks as a gate.
    fn checks(&self, number: u64) -> Result<Gate, String>;
    /// Ask GitHub to merge main into PR `number`.
    fn update_branch(&self, number: u64) -> Result<(), String>;
    /// Squash-merge PR `number` at `head`.
    fn merge(&self, number: u64, head: &str) -> Result<(), String>;
    /// Point PR `number` at main.
    fn retarget(&self, number: u64) -> Result<(), String>;
    /// Delete `branch` on GitHub.
    fn delete_branch(&self, branch: &str) -> Result<(), String>;
    /// Re-publish `/`, when the repository has a serve worktree.
    fn serve(&self) -> Result<(), String>;
    /// Wait `duration`.
    fn wait(&self, duration: Duration);
    /// Seconds since the epoch.
    fn now_secs(&self) -> u64;
}
