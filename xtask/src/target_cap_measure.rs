//! How much the capped build directory holds (`target_cap.rs` says what
//! is counted and why). Every directory or file that exists but cannot
//! be read is refused, naming it; only an entry a build removed while it
//! was listed counts as nothing.

use std::io::ErrorKind;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use super::COUNTED;

/// Bytes allocated to the counted files under `target`.
///
/// # Errors
/// A directory or file that could not be read.
pub fn measure(target: &Path) -> Result<u64, String> {
    let mut total = 0;
    for profile in profiles(target)? {
        for (name, depth) in COUNTED {
            total += files_bytes(&profile.join(name), depth)?;
        }
    }
    Ok(total)
}

/// `target/*` and `target/*/*`: every directory a profile can be.
fn profiles(target: &Path) -> Result<Vec<PathBuf>, String> {
    let mut found = Vec::new();
    for level_one in subdirs(target)? {
        found.extend(subdirs(&level_one)?);
        found.push(level_one);
    }
    Ok(found)
}

/// The directories directly in `dir` (not links); none when `dir` is
/// gone.
fn subdirs(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut found = Vec::new();
    for (path, meta) in listed(dir)? {
        if meta.is_dir() {
            found.push(path);
        }
    }
    Ok(found)
}

/// The allocated bytes of the files in `dir` down to `depth` levels.
fn files_bytes(dir: &Path, depth: usize) -> Result<u64, String> {
    let mut total = 0;
    for (path, meta) in listed(dir)? {
        if meta.is_dir() && depth > 1 {
            total += files_bytes(&path, depth - 1)?;
        } else if meta.is_file() {
            total += meta.blocks() * 512;
        }
    }
    Ok(total)
}

/// Each entry of `dir` with its own metadata (a link's, not its
/// target's); nothing when `dir` does not exist.
fn listed(dir: &Path) -> Result<Vec<(PathBuf, std::fs::Metadata)>, String> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(unreadable(dir, &e)),
    };
    let mut found = Vec::new();
    for entry in entries {
        let path = entry.map_err(|e| unreadable(dir, &e))?.path();
        match std::fs::symlink_metadata(&path) {
            Ok(meta) => found.push((path, meta)),
            // a build removed it meanwhile: it holds nothing now
            Err(e) if e.kind() == ErrorKind::NotFound => {}
            Err(e) => return Err(unreadable(&path, &e)),
        }
    }
    Ok(found)
}

fn unreadable(path: &Path, error: &std::io::Error) -> String {
    format!(
        "{}: {error}, so the build directory's size could not be measured; fix: check its permissions, or empty it",
        path.display()
    )
}
