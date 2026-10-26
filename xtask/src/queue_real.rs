//! The merge queue's real effects (`queue_ops::Ops`): gh, git in the
//! queue's own run worktree (reset hard to the exact head before every
//! action, and the head checked after checkout), `gate --pr` run by the
//! queue's own binary (main's, never the PR's), and the publish.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::gate_pr::Ended;
use crate::queue_config::QueueConfig;
use crate::queue_decide::{Clear, Gate};
use crate::queue_git::git;
use crate::queue_list::Listed;
use crate::queue_ops::{Now, Ops};
use crate::queue_remote;

const MAIN: &str = "origin/main";

/// How long a gate run by the queue may take: the gate lock's 40 min
/// wait (`gate_lock::WAIT`) and a touche gate's measured 4.5 to 8 min,
/// with room. Killed at the deadline, and the kill is logged.
const GATE_DEADLINE: Duration = Duration::from_mins(60);

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
        crate::queue_git::clear_holds(self.run(), MAIN, head, cleared)
    }

    fn touched(&self, head: &str) -> Result<Vec<String>, String> {
        let range = format!("{MAIN}...{head}");
        Ok(git(self.run(), &["diff", "--name-only", &range])?
            .lines()
            .map(str::to_owned)
            .collect())
    }

    fn rebased(&self, head: &str) -> Result<Option<String>, String> {
        self.at(head)?;
        crate::queue_git::rebase_clean(self.run(), MAIN, head)
    }

    fn push(&self, branch: &str, known: &str, new: &str) -> Result<(), String> {
        queue_remote::push_lease(self.run(), branch, known, new)
    }

    fn gate(&self, number: u64, head: &str) -> Result<Ended, String> {
        self.at(head)?;
        crate::queue_pins::move_siblings(self.run())?;
        let exe = std::env::current_exe().map_err(|e| format!("the queue's own path: {e}"))?;
        gate_child(gate_command(&exe, self.run(), number))
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
#[must_use]
pub fn gate_command(exe: &PathBuf, run: &Path, number: u64) -> Command {
    let mut command = Command::new(exe);
    command
        .args(["gate", "--pr", &number.to_string()])
        .current_dir(run);
    command
}

/// Run the gate child within `GATE_DEADLINE`; how it ended.
fn gate_child(mut command: Command) -> Result<Ended, String> {
    let mut child = command
        .stdin(Stdio::null())
        .spawn()
        .map_err(|e| format!("could not start the gate: {e}"))?;
    let started = Instant::now();
    while started.elapsed() <= GATE_DEADLINE {
        if let Some(status) = child
            .try_wait()
            .map_err(|e| format!("waiting for the gate: {e}"))?
        {
            return ended(status);
        }
        std::thread::sleep(Duration::from_secs(1));
    }
    let _ = child.kill();
    let _ = child.wait();
    Err(format!(
        "the gate did not finish within {GATE_DEADLINE:?}; killed"
    ))
}

/// An exit status as a code or the signal that ended it.
fn ended(status: std::process::ExitStatus) -> Result<Ended, String> {
    use std::os::unix::process::ExitStatusExt;
    match (status.code(), status.signal()) {
        (Some(code), _) => Ok(Ended::Code(code)),
        (None, Some(signal)) => Ok(Ended::Signal(signal)),
        (None, None) => Err("the gate ended with neither a code nor a signal".into()),
    }
}
