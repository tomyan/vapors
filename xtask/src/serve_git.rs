//! What `cargo xtask serve` asks of git: `origin/main` fetched, and the
//! checkout clean and at it, so the build served is main's.

use std::process::Command;
use std::time::Duration;

use crate::deadline::{Finished, run_with_deadline};
use crate::gate::git;

/// `git fetch origin main` took 0.66 s here (2026-10-25); 60 s leaves
/// room for a slow network and fails loudly past it.
const FETCH_DEADLINE: Duration = Duration::from_secs(60);

/// Fetch `origin main` into the checkout at `dir`.
///
/// # Errors
///
/// git failed or passed the deadline, naming it.
pub fn fetch_main(dir: &str) -> Result<(), String> {
    let mut command = Command::new("git");
    command.args(["-C", dir, "fetch", "-q", "origin", "main"]);
    for name in ["GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE"] {
        command.env_remove(name);
    }
    match run_with_deadline(command, FETCH_DEADLINE)? {
        Finished::Exited { success: true, .. } => Ok(()),
        Finished::Exited { output, .. } => Err(format!(
            "git fetch origin main failed:\n{output}fix: check the remote, then serve again"
        )),
        Finished::TimedOut { .. } => Err(format!(
            "git fetch origin main did not finish within {}s",
            FETCH_DEADLINE.as_secs()
        )),
    }
}

/// The checkout at `dir` is clean and at `origin/main`.
///
/// # Errors
///
/// Another commit or a changed tree, naming which.
pub fn require_main(dir: &str) -> Result<(), String> {
    let head = git(dir, &["rev-parse", "HEAD"])?;
    let main = git(dir, &["rev-parse", "origin/main"])?;
    let dirty = !git(dir, &["status", "--porcelain"])?.is_empty();
    main_refusal(&head, &main, dirty).map_or(Ok(()), Err)
}

/// Why a checkout at `head` cannot be served as main, or `None`.
#[must_use]
pub fn main_refusal(head: &str, main: &str, dirty: bool) -> Option<String> {
    if head != main {
        return Some(format!(
            "HEAD is {head}, not origin/main {main}; fix: run this from a clean worktree at origin/main"
        ));
    }
    dirty.then(|| "the tree has changes, so the build is not main's; fix: commit or stash them, or use a clean worktree".to_owned())
}

#[cfg(test)]
#[path = "serve_git_tests.rs"]
mod tests;
