//! The merge queue's git questions about one PR, in the queue's own
//! worktree: whether its head contains main and whether it rebases onto
//! main without a conflict, and the git calls the clear check
//! (`queue_clear.rs`) uses. These are local plumbing calls, bounded by the
//! disk; fetching and pushing, which go over the network, have their
//! deadline in `queue_remote.rs`.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

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

/// The paths `head` changes since it left `base`, a rename listed as
/// both of its paths.
///
/// # Errors
///
/// git's refusal, as text.
pub fn changed_paths(dir: &Path, base: &str, head: &str) -> Result<Vec<String>, String> {
    let range = format!("{base}...{head}");
    let listed = git(dir, &["diff", "--name-only", "--no-renames", &range])?;
    Ok(listed.lines().map(str::to_owned).collect())
}

/// git in `dir` with `input` on its stdin, `GIT_INDEX_FILE` as `index`.
pub fn piped(
    dir: &Path,
    args: &[&str],
    input: &str,
    index: Option<&str>,
) -> Result<Output, String> {
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

pub fn git_output(dir: &Path, args: &[&str]) -> Result<Output, String> {
    command(dir, args).output().map_err(|e| format!("git: {e}"))
}

/// git in `dir`, hook variables stripped (CLAUDE.md).
pub fn command(dir: &Path, args: &[&str]) -> Command {
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

pub fn failure(args: &[&str], output: &Output) -> String {
    format!(
        "git {}: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr).trim()
    )
}

#[cfg(test)]
#[path = "queue_git_tests.rs"]
mod tests;
