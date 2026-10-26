//! Target directories of a merged branch's worktrees, which the queue
//! empties after the merge (`queue_targets_empty.rs`), as touche's
//! CLAUDE.md asks of whoever merges. A worktree's `target`, and those of
//! its sibling repositories (touche's `../vapors`), are links into a
//! targets root (`~/.cache/touche-targets/<name>`).
//!
//! A target qualifies only when it is a link, by absolute path, to a real
//! directory (not itself a link) whose canonical parent is one of the
//! canonical roots in `[queue] targets`. Anything else is left alone and
//! named. Declared limits: a worktree is matched by its branch's name
//! alone, so a stale or prunable worktree on that name is emptied too
//! (the cost is a rebuild inside the roots); and a link swapped between
//! this check and the removal is not caught.

use std::path::{Path, PathBuf};

/// The worktrees on `branch` in `git worktree list --porcelain`'s output.
#[must_use]
pub fn worktrees_on(porcelain: &str, branch: &str) -> Vec<PathBuf> {
    let wanted = format!("branch refs/heads/{branch}");
    entries(porcelain)
        .filter(|(_, entry)| entry.lines().any(|line| line == wanted))
        .map(|(path, _)| path)
        .collect()
}

/// Every worktree in the porcelain list.
#[must_use]
pub fn all_worktrees(porcelain: &str) -> Vec<PathBuf> {
    entries(porcelain).map(|(path, _)| path).collect()
}

fn entries(porcelain: &str) -> impl Iterator<Item = (PathBuf, &str)> {
    porcelain.split("\n\n").filter_map(|entry| {
        let path = entry
            .lines()
            .find_map(|line| line.strip_prefix("worktree "))?;
        Some((PathBuf::from(path), entry))
    })
}

/// The target directories of `worktree` and its sibling repositories,
/// each a canonical directory directly in one of `roots` (canonical) or,
/// when it is not, why it is left alone.
#[must_use]
pub fn target_dirs(worktree: &Path, roots: &[PathBuf]) -> Vec<Result<PathBuf, String>> {
    match sibling_targets(worktree) {
        Ok(targets) => targets
            .iter()
            .map(|target| in_a_root(target, roots))
            .collect(),
        Err(why) => vec![Err(why)],
    }
}

/// The `target` of each repository beside `worktree`, itself included,
/// that has one.
fn sibling_targets(worktree: &Path) -> Result<Vec<PathBuf>, String> {
    let parent = worktree
        .parent()
        .ok_or_else(|| format!("{} has no parent directory", worktree.display()))?;
    let named = |e: std::io::Error| format!("{}: {e}", parent.display());
    let mut targets = Vec::new();
    for entry in std::fs::read_dir(parent).map_err(named)? {
        let repository = entry.map_err(named)?.path();
        let target = repository.join("target");
        if repository.join(".git").exists() && target.symlink_metadata().is_ok() {
            targets.push(target);
        }
    }
    Ok(targets)
}

/// The directory `target` links to, when it qualifies; otherwise why it
/// is left alone.
fn in_a_root(target: &Path, roots: &[PathBuf]) -> Result<PathBuf, String> {
    let alone = |why: String| Err(format!("{} {why}; left alone", target.display()));
    let Ok(link) = std::fs::read_link(target) else {
        return alone("is a directory, not a link into a targets root".into());
    };
    if link.is_relative() {
        return alone(format!("links to the relative path {}", link.display()));
    }
    match destination(&link, roots) {
        Ok(dir) => Ok(dir),
        Err(why) => alone(format!("links to {}, {why}", link.display())),
    }
}

/// `link`'s canonical directory, when it is a real directory directly in
/// a root; otherwise what it is.
fn destination(link: &Path, roots: &[PathBuf]) -> Result<PathBuf, String> {
    let meta =
        std::fs::symlink_metadata(link).map_err(|e| format!("which cannot be read ({e})"))?;
    if meta.is_symlink() {
        return Err("which is itself a link".into());
    }
    let real = link
        .canonicalize()
        .map_err(|e| format!("which cannot be resolved ({e})"))?;
    let in_root = real
        .parent()
        .is_some_and(|parent| roots.iter().any(|root| root == parent));
    if meta.is_dir() && in_root {
        return Ok(real);
    }
    Err("not a directory directly in a targets root".into())
}

#[cfg(test)]
#[path = "queue_targets_tests.rs"]
mod tests;
