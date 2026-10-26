//! The merge queue's git questions about one PR, in the queue's own
//! worktree: whether its head contains main, whether its review clear
//! still holds (the same patch-id set, `git patch-id --stable`, as the
//! cleared commit had over main), and whether it rebases onto main
//! without a conflict. These are local plumbing calls, bounded by the
//! disk; fetching and pushing, which go over the network, have their
//! deadline in `queue_remote.rs`.

use std::collections::BTreeMap;
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

/// Whether the clear of `cleared` still describes `head`. Each commit of
/// `head` over `main` must be the same patch as one of `cleared`'s, and
/// each of `cleared`'s left over must already be in `main` (a parent PR
/// squashed into it). Patches are compared without context (`-U0`), so
/// main changing lines near a hunk keeps the clear; `git patch-id`
/// ignores line numbers and whitespace, so a change of indentation
/// alone keeps it too (a declared limit).
///
/// # Errors
///
/// A commit git cannot read.
pub fn clear_holds(dir: &Path, main: &str, head: &str, cleared: &str) -> Result<Clear, String> {
    let now = patch_ids(dir, main, head)?;
    let then = patch_ids(dir, main, cleared)?;
    if now.keys().any(|id| !then.contains_key(id)) {
        return Ok(Clear::Changed);
    }
    for (id, commit) in &then {
        if !now.contains_key(id) && !in_main(dir, main, commit)? {
            return Ok(Clear::Changed);
        }
    }
    Ok(Clear::Holds)
}

/// Whether `main` already holds `commit`'s change: its diff applies in
/// reverse to main's tree (in a scratch index, touching no worktree).
fn in_main(dir: &Path, main: &str, commit: &str) -> Result<bool, String> {
    let index = std::env::temp_dir().join(format!("xtask-queue-index-{}", std::process::id()));
    let index_text = index.display().to_string();
    let read = command(dir, &["read-tree", main])
        .env("GIT_INDEX_FILE", &index_text)
        .output();
    read.map_err(|e| format!("git read-tree: {e}"))?;
    let diff = git(dir, &["show", "--format=", "--binary", commit])?;
    let applied = piped(
        dir,
        &["apply", "--check", "--reverse", "--cached"],
        &diff,
        Some(&index_text),
    );
    let _ = std::fs::remove_file(&index);
    applied.map(|output| output.status.success())
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

/// Each commit's patch id over `base..tip`, to its commit.
fn patch_ids(dir: &Path, base: &str, tip: &str) -> Result<BTreeMap<String, String>, String> {
    let range = format!("{base}..{tip}");
    let mut ids = BTreeMap::new();
    for commit in git(dir, &["rev-list", &range])?.lines() {
        let id = patch_id(dir, commit)?;
        if !id.is_empty() {
            ids.insert(id, commit.to_owned());
        }
    }
    Ok(ids)
}

/// `commit`'s patch id, without context; empty for a commit with no diff.
fn patch_id(dir: &Path, commit: &str) -> Result<String, String> {
    let diff = git(dir, &["show", "--format=", "-U0", commit])?;
    let output = piped(dir, &["patch-id", "--stable"], &diff, None)?;
    let text = String::from_utf8_lossy(&output.stdout);
    Ok(text
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_owned())
}

/// git in `dir` with `input` on its stdin, `GIT_INDEX_FILE` as `index`.
fn piped(dir: &Path, args: &[&str], input: &str, index: Option<&str>) -> Result<Output, String> {
    let mut command = command(dir, args);
    if let Some(index) = index {
        command.env("GIT_INDEX_FILE", index);
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("git {}: {e}", args.join(" ")))?;
    let mut stdin = child.stdin.take().ok_or("git's input")?;
    stdin
        .write_all(input.as_bytes())
        .map_err(|e| format!("git {}: {e}", args.join(" ")))?;
    drop(stdin);
    child
        .wait_with_output()
        .map_err(|e| format!("git {}: {e}", args.join(" ")))
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
