//! Whether a PR's review clear still describes its head, in the queue's
//! own worktree.
//!
//! Each commit of the head over main, oldest first and merges aside, must
//! be the same patch as a cleared commit not already matched, so a patch
//! added twice is seen. Patches are compared with one line of context
//! (`-U1`): enough to place a hunk, so an insertion moved elsewhere
//! breaks the clear, while main changing a line two away keeps it.
//! `git patch-id` ignores line numbers and whitespace, so a change of
//! indentation alone keeps the clear (a declared limit). Each cleared
//! commit left unmatched must already be in main (a parent PR squashed
//! into it). Each merge commit must be exactly git's own merge of its
//! parents (`git merge-tree --write-tree`), so a merge that resolves a
//! conflict, on either side, breaks the clear. A lapse says why, naming
//! the commit.

use std::path::Path;

use crate::queue_decide::Clear;
use crate::queue_git::{command, failure, git, git_output, piped};

/// Whether the clear of `cleared` still describes `head`.
///
/// # Errors
///
/// A commit git cannot read, or git failing other than by a conflict.
pub fn clear_holds(dir: &Path, main: &str, head: &str, cleared: &str) -> Result<Clear, String> {
    if let Some(merge) = resolved_merge(dir, main, head, cleared)? {
        let why = format!("merge {merge} differs from git's own merge of its parents");
        return Ok(Clear::Changed(why));
    }
    let unmatched = match match_patches(dir, main, head, cleared)? {
        Ok(unmatched) => unmatched,
        Err(commit) => {
            return Ok(Clear::Changed(format!(
                "{commit} is not a patch the clear covers"
            )));
        }
    };
    for (_, commit) in &unmatched {
        if !in_main(dir, main, commit)? {
            let why = format!("cleared {commit} is in neither the head nor main");
            return Ok(Clear::Changed(why));
        }
    }
    Ok(Clear::Holds)
}

/// Match each of `head`'s patches, oldest first, to one of `cleared`'s
/// not yet matched: the cleared ones left over, or the first commit of
/// `head` with no match.
fn match_patches(
    dir: &Path,
    main: &str,
    head: &str,
    cleared: &str,
) -> Result<Result<Vec<(String, String)>, String>, String> {
    let mut unmatched = patch_ids(dir, main, cleared)?;
    for (id, commit) in patch_ids(dir, main, head)? {
        let Some(at) = unmatched
            .iter()
            .position(|(cleared_id, _)| *cleared_id == id)
        else {
            return Ok(Err(commit));
        };
        unmatched.remove(at);
    }
    Ok(Ok(unmatched))
}

/// The first merge commit over `main`, after `cleared` (the reviewer saw
/// the merges before it), whose tree is not git's clean merge of its two
/// parents.
fn resolved_merge(
    dir: &Path,
    main: &str,
    head: &str,
    cleared: &str,
) -> Result<Option<String>, String> {
    let (range, seen) = (format!("{main}..{head}"), format!("^{cleared}"));
    for merge in git(dir, &["rev-list", "--merges", &range, &seen])?.lines() {
        if !clean_merge(dir, merge)? {
            return Ok(Some(merge.to_owned()));
        }
    }
    Ok(None)
}

/// Whether `merge` has two parents and their automatic merge, with no
/// conflict, is its tree.
fn clean_merge(dir: &Path, merge: &str) -> Result<bool, String> {
    let line = git(dir, &["rev-list", "--parents", "-n", "1", merge])?;
    let parents: Vec<&str> = line.split_whitespace().skip(1).collect();
    let [first, second] = parents[..] else {
        return Ok(false);
    };
    let args = ["merge-tree", "--write-tree", first, second];
    let output = git_output(dir, &args)?;
    match output.status.code() {
        Some(0) => {
            let tree = git(dir, &["rev-parse", &format!("{merge}^{{tree}}")])?;
            Ok(String::from_utf8_lossy(&output.stdout).lines().next() == Some(tree.as_str()))
        }
        Some(1) => Ok(false),
        _ => Err(failure(&args, &output)),
    }
}

/// Whether `main` already holds `commit`'s change: its diff applies in
/// reverse to main's tree (in a scratch index, touching no worktree).
fn in_main(dir: &Path, main: &str, commit: &str) -> Result<bool, String> {
    let index = std::env::temp_dir().join(format!("xtask-queue-index-{}", std::process::id()));
    let index_text = index.display().to_string();
    let applied = read_tree(dir, main, &index_text).and_then(|()| {
        let diff = git(dir, &["show", "--format=", "--binary", commit])?;
        let args = ["apply", "--check", "--reverse", "--cached"];
        piped(dir, &args, &diff, Some(&index_text))
    });
    let _ = std::fs::remove_file(&index);
    applied.map(|output| output.status.success())
}

/// Read `main`'s tree into the scratch index `index`, judged by git's
/// exit code.
fn read_tree(dir: &Path, main: &str, index: &str) -> Result<(), String> {
    let read = command(dir, &["read-tree", main])
        .env("GIT_INDEX_FILE", index)
        .output()
        .map_err(|e| format!("git read-tree into {index}: {e}"))?;
    if read.status.success() {
        return Ok(());
    }
    Err(format!(
        "{} (index {index})",
        failure(&["read-tree", main], &read)
    ))
}

/// Each non-merge commit's patch id over `base..tip`, oldest first, with
/// its commit; a commit with no diff has none.
fn patch_ids(dir: &Path, base: &str, tip: &str) -> Result<Vec<(String, String)>, String> {
    let range = format!("{base}..{tip}");
    let mut ids = Vec::new();
    for commit in git(dir, &["rev-list", "--reverse", "--no-merges", &range])?.lines() {
        let id = patch_id(dir, commit)?;
        if !id.is_empty() {
            ids.push((id, commit.to_owned()));
        }
    }
    Ok(ids)
}

/// `commit`'s patch id with one line of context; empty with no diff.
fn patch_id(dir: &Path, commit: &str) -> Result<String, String> {
    let diff = git(dir, &["show", "--format=", "-U1", commit])?;
    let output = piped(dir, &["patch-id", "--stable"], &diff, None)?;
    let text = String::from_utf8_lossy(&output.stdout);
    Ok(text
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_owned())
}

#[cfg(test)]
#[path = "queue_clear_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "queue_clear_merge_tests.rs"]
mod merge_tests;
