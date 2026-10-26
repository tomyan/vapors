//! The merge queue's real effects (`queue_ops::Ops`): gh, git in the
//! queue's own run worktree (reset hard to the exact head before every
//! action, and the head checked after checkout), `gate --pr` run by the
//! queue's own binary (main's, never the PR's), and the publish.

use std::path::Path;
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::queue_config::QueueConfig;
use crate::queue_decide::{Clear, Gate};
use crate::queue_gate_child::GATE_DEADLINE;
use crate::queue_git::git;
use crate::queue_list::Listed;
use crate::queue_ops::{Gated, Now, Ops};
use crate::queue_remote;

const MAIN: &str = "origin/main";

/// The real effects, for the repository `config` names.
pub struct Real<'a> {
    /// The queue's settings.
    pub config: &'a QueueConfig,
}

impl Real<'_> {
    fn run(&self) -> &Path {
        &self.config.run
    }

    /// The run worktree reset hard to `head`, checked out detached.
    fn at(&self, head: &str) -> Result<(), String> {
        let _ = git(self.run(), &["rebase", "--abort"]);
        git(self.run(), &["reset", "-q", "--hard"])?;
        git(self.run(), &["checkout", "-q", "--detach", head])?;
        let now = git(self.run(), &["rev-parse", "HEAD"])?;
        if now != head {
            return Err(format!(
                "the run worktree is at {now} after checking out {head}"
            ));
        }
        Ok(())
    }
}

impl Ops for Real<'_> {
    fn open_prs(&self) -> Result<Vec<Listed>, String> {
        crate::queue_list::open_prs(&self.config.repo)
    }

    fn comments(&self, number: u64) -> Result<Vec<String>, String> {
        crate::queue_list::comments(&self.config.repo, number)
    }

    fn now(&self, number: u64) -> Result<Now, String> {
        crate::queue_list::pr_now(&self.config.repo, number)
    }

    fn fetch(&self, refs: &[&str]) -> Result<(), String> {
        queue_remote::fetch(self.run(), refs)
    }

    fn contains_main(&self, head: &str) -> Result<bool, String> {
        crate::queue_git::contains(self.run(), MAIN, head)
    }

    fn clear_holds(&self, head: &str, cleared: &str) -> Result<Clear, String> {
        crate::queue_clear::clear_holds(self.run(), MAIN, head, cleared)
    }

    fn touched(&self, head: &str) -> Result<Vec<String>, String> {
        crate::queue_git::changed_paths(self.run(), MAIN, head)
    }

    fn rebased(&self, head: &str) -> Result<Option<String>, String> {
        self.at(head)?;
        crate::queue_git::rebase_clean(self.run(), MAIN, head)
    }

    fn push(&self, branch: &str, known: &str, new: &str) -> Result<(), String> {
        queue_remote::push_lease(self.run(), branch, known, new)
    }

    fn gate(&self, number: u64, head: &str) -> Result<Gated, String> {
        self.at(head)?;
        crate::queue_pins::move_siblings(self.run())?;
        let out = format!("{}.gate.out", self.config.log.display());
        crate::queue_gate_child::run(
            gate_command(self.run(), number)?,
            GATE_DEADLINE,
            Path::new(&out),
        )
    }

    fn checks(&self, number: u64) -> Result<Gate, String> {
        crate::queue_checks::checks(&self.config.repo, number)
    }

    fn update_branch(&self, number: u64) -> Result<(), String> {
        crate::queue_checks::update_branch(&self.config.repo, number)
    }

    fn merge(&self, number: u64, head: &str) -> Result<(), String> {
        queue_remote::merge(&self.config.repo, number, head)
    }

    fn retarget(&self, number: u64) -> Result<(), String> {
        queue_remote::retarget(&self.config.repo, number)
    }

    fn delete_branch(&self, branch: &str) -> Result<(), String> {
        queue_remote::delete_branch(&self.config.repo, branch)
    }

    fn serve(&self) -> Result<(), String> {
        self.config
            .serve
            .as_deref()
            .map_or(Ok(()), crate::queue_serve::serve)
    }

    fn wait(&self, duration: Duration) {
        std::thread::sleep(duration);
    }

    fn now_secs(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs())
    }
}

/// `<the queue's own binary> gate --pr N`, run in the run worktree: main's
/// code finds the PR, runs the PR's ci.yml steps, and posts the result.
/// The binary is this process's own (`current_exe`), never one the PR
/// builds.
///
/// # Errors
///
/// This process's path cannot be read.
pub fn gate_command(run: &Path, number: u64) -> Result<Command, String> {
    let exe = std::env::current_exe().map_err(|e| format!("the queue's own path: {e}"))?;
    let mut command = Command::new(exe);
    command
        .args(["gate", "--pr", &number.to_string()])
        .current_dir(run);
    Ok(command)
}

#[cfg(test)]
#[path = "queue_real_tests.rs"]
mod tests;
