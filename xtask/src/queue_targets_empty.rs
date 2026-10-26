//! Emptying a merged branch's targets (found by `queue_targets.rs`).
//! Before anything is removed, the directory is refused if it is `/`,
//! the home directory, or any worktree in the list: none of them is a
//! build's output, whatever the roots say. Emptying keeps the directory,
//! so the links to it stay valid; links inside it are removed, never
//! followed. It has no deadline (a removal of up to ~60 GB on a local
//! disk); each emptying logs how long it took, to set one from.

use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::queue_targets::{all_worktrees, target_dirs, worktrees_on};

/// Empty the targets of every worktree on `branch` in `porcelain`; a log
/// line for each directory emptied, left alone, refused, or failing.
#[must_use]
pub fn empty_on(porcelain: &str, branch: &str, roots: &[PathBuf], home: &Path) -> Vec<String> {
    let worktrees = worktrees_on(porcelain, branch);
    if worktrees.is_empty() {
        return vec![format!("no worktree is on {branch}; no targets emptied")];
    }
    let mut never: Vec<(PathBuf, &str)> = vec![(home.to_path_buf(), "the home directory")];
    never.extend(
        all_worktrees(porcelain)
            .into_iter()
            .map(|w| (w, "a worktree")),
    );
    worktrees
        .iter()
        .flat_map(|worktree| target_dirs(worktree, roots))
        .map(|dir| dir.map_or_else(|why| why, |dir| empty_unless_forbidden(&dir, &never)))
        .collect()
}

fn empty_unless_forbidden(dir: &Path, never: &[(PathBuf, &str)]) -> String {
    if let Some(what) = forbidden_as(dir, never) {
        return format!("refused to empty {}: it {what}", dir.display());
    }
    let started = Instant::now();
    match empty(dir) {
        Ok(removed) => {
            let entries = if removed == 1 { "entry" } else { "entries" };
            let took = started.elapsed().as_secs_f64();
            format!(
                "emptied {} ({removed} top-level {entries}, in {took:.1} s)",
                dir.display()
            )
        }
        Err(why) => why,
    }
}

/// Why `dir` must never be emptied: it is `/`, or it is or holds one of
/// `never` (compared canonically), as a root holding worktrees would.
#[must_use]
pub fn forbidden_as(dir: &Path, never: &[(PathBuf, &str)]) -> Option<String> {
    if dir == Path::new("/") {
        return Some("is the root directory".into());
    }
    let canonical = |path: &Path| path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let dir = canonical(dir);
    never.iter().find_map(|(path, what)| {
        let path = canonical(path);
        if path == dir {
            Some(format!("is {what}"))
        } else {
            path.starts_with(&dir).then(|| format!("holds {what}"))
        }
    })
}

/// Remove everything in `dir`, keeping `dir`; how many top-level entries
/// went.
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
#[path = "queue_targets_empty_tests.rs"]
mod tests;
