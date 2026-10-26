//! Target directories of a merged branch's worktrees, emptied after the
//! merge, as touche's CLAUDE.md asks of whoever merges: a worktree's
//! `target`, and its sibling repositories' (touche's `../vapors`), are
//! symlinks into a targets root (`~/.cache/touche-targets/<name>`), and
//! emptying them keeps the directories so the links stay valid. Only a
//! link to a directory directly in a root in `[queue] targets` is
//! emptied; anything else is left alone and named.

use std::path::{Path, PathBuf};

/// Empty the targets of every worktree on `branch` in `porcelain`; a log
/// line for each directory emptied, left alone, or failing.
#[must_use]
pub fn empty_on(porcelain: &str, branch: &str, roots: &[PathBuf]) -> Vec<String> {
    let worktrees = worktrees_on(porcelain, branch);
    if worktrees.is_empty() {
        return vec![format!("no worktree is on {branch}; no targets emptied")];
    }
    worktrees
        .iter()
        .flat_map(|worktree| target_dirs(worktree, roots))
        .map(
            |dir| match dir.and_then(|dir| empty(&dir).map(|n| (dir, n))) {
                Ok((dir, removed)) => format!("emptied {} ({removed} entries)", dir.display()),
                Err(why) => why,
            },
        )
        .collect()
}

/// The worktrees on `branch` in `git worktree list --porcelain`'s output.
#[must_use]
pub fn worktrees_on(porcelain: &str, branch: &str) -> Vec<PathBuf> {
    let wanted = format!("branch refs/heads/{branch}");
    porcelain
        .split("\n\n")
        .filter(|entry| entry.lines().any(|line| line == wanted))
        .filter_map(|entry| {
            entry
                .lines()
                .find_map(|line| line.strip_prefix("worktree "))
        })
        .map(PathBuf::from)
        .collect()
}

/// The target directories of `worktree` and its siblings, each a
/// directory in one of `roots` or, when it is not, why it is left alone.
#[must_use]
pub fn target_dirs(worktree: &Path, roots: &[PathBuf]) -> Vec<Result<PathBuf, String>> {
    let Some(parent) = worktree.parent() else {
        return vec![Err(format!(
            "{} has no parent directory",
            worktree.display()
        ))];
    };
    let named = |e: std::io::Error| format!("{}: {e}", parent.display());
    let siblings = match std::fs::read_dir(parent) {
        Ok(siblings) => siblings,
        Err(e) => return vec![Err(named(e))],
    };
    siblings
        .map(|entry| entry.map(|e| e.path().join("target")).map_err(named))
        .filter(|target| {
            target
                .as_ref()
                .map_or(true, |t| t.symlink_metadata().is_ok())
        })
        .map(|target| target.and_then(|t| in_a_root(&t, roots)))
        .collect()
}

/// The directory `target` links to, when it is directly in one of
/// `roots`; otherwise why it is left alone.
fn in_a_root(target: &Path, roots: &[PathBuf]) -> Result<PathBuf, String> {
    let Ok(link) = std::fs::read_link(target) else {
        return Err(format!(
            "{} is a directory, not a link into a targets root; left alone",
            target.display()
        ));
    };
    let in_root = link
        .parent()
        .is_some_and(|parent| roots.iter().any(|root| root == parent));
    if in_root && link.is_dir() {
        return Ok(link);
    }
    Err(format!(
        "{} links to {}, not a directory in a targets root; left alone",
        target.display(),
        link.display()
    ))
}

/// Remove everything in `dir`, keeping `dir`; how many entries went.
///
/// # Errors
///
/// An entry that cannot be read or removed, naming it.
pub fn empty(dir: &Path) -> Result<usize, String> {
    let named = |path: &Path, e: std::io::Error| format!("{}: {e}", path.display());
    let mut removed = 0;
    for entry in std::fs::read_dir(dir).map_err(|e| named(dir, e))? {
        let path = entry.map_err(|e| named(dir, e))?.path();
        remove(&path).map_err(|e| named(&path, e))?;
        removed += 1;
    }
    Ok(removed)
}

/// Remove `path`; a link is removed itself, never followed.
fn remove(path: &Path) -> std::io::Result<()> {
    if path.symlink_metadata()?.is_dir() {
        std::fs::remove_dir_all(path)
    } else {
        std::fs::remove_file(path)
    }
}

#[cfg(test)]
#[path = "queue_targets_tests.rs"]
mod tests;
