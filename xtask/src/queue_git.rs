//! The merge queue's git questions about one PR, in the queue's own
//! worktree: whether its head contains main, whether its review clear
//! still holds (the same patch-id set, `git patch-id --stable`, as the
//! cleared commit had over main), and whether it rebases onto main
//! without a conflict. These are local plumbing calls, bounded by the
//! disk; fetching and pushing, which go over the network, have their
//! deadline in `queue_remote.rs`.

use std::collections::BTreeSet;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

use crate::queue_decide::Clear;

/// Whether `head` contains `base`.
///
/// # Errors
///
/// git's refusal (an unknown commit), as text.
pub fn contains(dir: &Path, base: &str, head: &str) -> Result<bool, String> {
    let output = git_output(dir, &["merge-base", "--is-ancestor", base, head])?;
    match output.status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        _ => Err(failure(
            &["merge-base", "--is-ancestor", base, head],
            &output,
        )),
    }
}

/// Whether the clear of `cleared` still describes `head`: the commits
/// each has over `main` are the same patches.
///
/// # Errors
///
/// A commit git cannot read.
pub fn clear_holds(dir: &Path, main: &str, head: &str, cleared: &str) -> Result<Clear, String> {
    let now = patch_ids(dir, main, head)?;
    let then = patch_ids(dir, main, cleared)?;
    Ok(if now == then {
        Clear::Holds
    } else {
        Clear::Changed
    })
}

/// `head` rebased onto `main` and checked out detached: the new head, or
/// none when the rebase conflicts (it is then aborted).
///
/// # Errors
///
/// git failing other than by a conflict.
pub fn rebase_clean(dir: &Path, main: &str, head: &str) -> Result<Option<String>, String> {
    git(dir, &["checkout", "-q", "--detach", head])?;
    let rebase = git_output(dir, &["rebase", "-q", main])?;
    if rebase.status.success() {
        return git(dir, &["rev-parse", "HEAD"]).map(Some);
    }
    git(dir, &["rebase", "--abort"])?;
    Ok(None)
}

/// Each commit's patch id over `base..tip`.
fn patch_ids(dir: &Path, base: &str, tip: &str) -> Result<BTreeSet<String>, String> {
    let range = format!("{base}..{tip}");
    git(dir, &["rev-list", &range])?
        .lines()
        .map(|commit| patch_id(dir, commit))
        .filter(|id| !matches!(id, Ok(id) if id.is_empty()))
        .collect()
}

/// `commit`'s patch id; empty for a commit with no diff.
fn patch_id(dir: &Path, commit: &str) -> Result<String, String> {
    let diff = git(dir, &["show", "--format=", commit])?;
    let mut child = command(dir, &["patch-id", "--stable"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| format!("git patch-id: {e}"))?;
    let mut stdin = child.stdin.take().ok_or("git patch-id's input")?;
    stdin
        .write_all(diff.as_bytes())
        .map_err(|e| format!("git patch-id: {e}"))?;
    drop(stdin);
    let output = child
        .wait_with_output()
        .map_err(|e| format!("git patch-id: {e}"))?;
    let text = String::from_utf8_lossy(&output.stdout);
    Ok(text
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_owned())
}

/// git in `dir`, its stdout trimmed; it must exit 0.
pub fn git(dir: &Path, args: &[&str]) -> Result<String, String> {
    let output = git_output(dir, args)?;
    if !output.status.success() {
        return Err(failure(args, &output));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn git_output(dir: &Path, args: &[&str]) -> Result<Output, String> {
    command(dir, args).output().map_err(|e| format!("git: {e}"))
}

/// git in `dir`, hook variables stripped (CLAUDE.md).
fn command(dir: &Path, args: &[&str]) -> Command {
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(dir)
        .args(args)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE");
    command
}

fn failure(args: &[&str], output: &Output) -> String {
    format!(
        "git {}: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr).trim()
    )
}

#[cfg(test)]
#[path = "queue_git_tests.rs"]
mod tests;
