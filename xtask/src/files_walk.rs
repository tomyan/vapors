//! Finding the files the length rule covers, and counting their lines.
//! Symlinks are followed: a symlinked file counts through its target, a
//! symlinked directory is walked once (cycles are cut by resolved
//! path), and a dangling symlink is refused naming it. Directories
//! named in `SKIPPED_DIRS` are never entered, at any depth; other
//! hidden directories (`.config`, `.cargo`) are ours and are walked.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Extensions the rule covers.
const EXTENSIONS: [&str; 5] = ["rs", "wgsl", "html", "js", "toml"];

/// Directory names never walked, at any depth: build output, git,
/// dependencies, fonts. Matched by exact component, so `crates/targets`
/// is walked.
const SKIPPED_DIRS: [&str; 4] = ["target", ".git", "node_modules", "fonts"];

/// Every covered file under `root`, as `crates/...` paths, sorted.
///
/// # Errors
///
/// A message naming a directory that could not be read, or a dangling
/// symlink, in the same path form as the findings.
pub fn workspace_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    files_skipping(root, &SKIPPED_DIRS)
}

/// Every covered file under `root` except build output and git, fonts
/// and `node_modules` included: for a check that must see every source.
///
/// # Errors
///
/// As [`workspace_files`].
pub fn all_source_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    files_skipping(root, &["target", ".git"])
}

fn files_skipping(root: &Path, skip: &[&str]) -> Result<Vec<PathBuf>, String> {
    let mut found = Vec::new();
    let mut visited = BTreeSet::new();
    walk(root, Path::new(""), skip, &mut found, &mut visited)?;
    found.sort();
    Ok(found)
}

fn walk(
    root: &Path,
    relative: &Path,
    skip: &[&str],
    found: &mut Vec<PathBuf>,
    visited: &mut BTreeSet<PathBuf>,
) -> Result<(), String> {
    let dir = root.join(relative);
    let resolved = dir
        .canonicalize()
        .map_err(|e| format!("{}: {e}", shown(relative)))?;
    if !visited.insert(resolved) {
        return Ok(()); // a symlink cycle, or two links to one directory
    }
    let entries = std::fs::read_dir(&dir).map_err(|e| format!("{}: {e}", shown(relative)))?;
    // In name order, so which of a directory and a link to it is walked
    // first does not depend on the file system.
    let mut names: Vec<_> = entries
        .map(|entry| entry.map(|e| e.file_name()).map_err(|e| e.to_string()))
        .collect::<Result<_, _>>()?;
    names.sort();
    for name in names {
        visit(root, &relative.join(name), skip, found, visited)?;
    }
    Ok(())
}

/// One entry: a directory is walked unless skipped, a file is kept when
/// covered; a symlink is resolved first and refused when it dangles.
fn visit(
    root: &Path,
    path: &Path,
    skip: &[&str],
    found: &mut Vec<PathBuf>,
    visited: &mut BTreeSet<PathBuf>,
) -> Result<(), String> {
    let full = root.join(path);
    let metadata = std::fs::metadata(&full).map_err(|e| {
        format!(
            "error: {}: dangling symlink or unreadable ({e}); fix: remove it or point it at a file",
            shown(path)
        )
    })?;
    if metadata.is_dir() {
        if !skipped(path, skip) {
            walk(root, path, skip, found, visited)?;
        }
    } else if metadata.is_file() && covered(path) {
        found.push(path.to_path_buf());
    }
    Ok(())
}

/// The path as findings print it: relative, `.` for the root.
fn shown(relative: &Path) -> String {
    if relative.as_os_str().is_empty() {
        ".".to_owned()
    } else {
        relative.display().to_string()
    }
}

fn skipped(path: &Path, skip: &[&str]) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| skip.contains(&n))
}

fn covered(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| EXTENSIONS.contains(&e))
}

/// Lines in `path`: the line terminators, plus one for a final line
/// without its terminator, so a 221-line file that ends without a
/// newline is 221.
///
/// # Errors
///
/// A message naming the file when it cannot be read.
#[allow(clippy::naive_bytecount)] // source files are small; a crate for this is not worth a dependency
pub fn count_lines(path: &Path) -> Result<usize, String> {
    let text = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let terminators = text.iter().filter(|&&b| b == b'\n').count();
    let unterminated = usize::from(text.last().is_some_and(|&b| b != b'\n'));
    Ok(terminators + unterminated)
}

#[cfg(test)]
#[path = "files_walk_tests.rs"]
mod tests;
