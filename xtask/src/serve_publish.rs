//! Putting a build where `cargo xtask serve` serves it. `dir` is a
//! symlink to a versioned copy beside it (`<dir>.<n>`); a new build is
//! copied to the next version, marked, and the link is swapped by one
//! rename, so the served path is never missing or half written. A
//! `dir` that exists without the marker publish writes is not ours and
//! is refused, never deleted.

use std::path::{Path, PathBuf};

/// The file publish writes into each copy it makes.
pub const MARKER: &str = ".xtask-served";

/// Make `dist` the build served at `dir`.
///
/// # Errors
///
/// `dir` is something publish did not make, or a copy, link or rename
/// failed; what was served before is still served.
pub fn publish(dist: &Path, dir: &Path) -> Result<PathBuf, String> {
    refuse_foreign(dir)?;
    let copy = next_copy(dir);
    if let Err(why) = fill(dist, &copy).and_then(|()| swap_link(dir, &copy)) {
        let _ = std::fs::remove_dir_all(&copy);
        return Err(why);
    }
    remove_old_copies(dir, &copy);
    Ok(copy)
}

/// `copy` holds `dist` and the marker.
fn fill(dist: &Path, copy: &Path) -> Result<(), String> {
    copy_dir(dist, copy)?;
    std::fs::write(copy.join(MARKER), "").map_err(|e| failed(copy, &e))
}

/// An existing `dir` must be a link to a copy publish made.
fn refuse_foreign(dir: &Path) -> Result<(), String> {
    let Ok(meta) = std::fs::symlink_metadata(dir) else {
        return Ok(());
    };
    if meta.file_type().is_symlink() && dir.join(MARKER).is_file() {
        return Ok(());
    }
    Err(format!(
        "{} exists and is not a copy `cargo xtask serve` made (no {MARKER}); fix: move it aside yourself, then serve again",
        dir.display()
    ))
}

/// More versioned copies than publish ever leaves (it keeps one).
const MAX_COPIES: u32 = 1000;

/// `<dir>.<n>`, the first `n` free.
fn next_copy(dir: &Path) -> PathBuf {
    let next = (1..MAX_COPIES)
        .find(|n| !versioned(dir, *n).exists())
        .unwrap_or(MAX_COPIES);
    versioned(dir, next)
}

fn versioned(dir: &Path, n: u32) -> PathBuf {
    let name = dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    dir.with_file_name(format!("{name}.{n}"))
}

/// Point `dir` at `copy` with a link made beside it and renamed over.
fn swap_link(dir: &Path, copy: &Path) -> Result<(), String> {
    let staged = dir.with_extension("link");
    let _ = std::fs::remove_file(&staged);
    std::os::unix::fs::symlink(copy, &staged).map_err(|e| failed(&staged, &e))?;
    std::fs::rename(&staged, dir).map_err(|e| failed(dir, &e))
}

/// Every other `<dir>.<n>` that carries the marker: no longer served.
fn remove_old_copies(dir: &Path, kept: &Path) {
    for n in 1..MAX_COPIES {
        let old = versioned(dir, n);
        if old != kept && old.join(MARKER).is_file() {
            let _ = std::fs::remove_dir_all(&old);
        }
    }
}

fn copy_dir(from: &Path, to: &Path) -> Result<(), String> {
    std::fs::create_dir_all(to).map_err(|e| failed(to, &e))?;
    for entry in std::fs::read_dir(from).map_err(|e| failed(from, &e))? {
        let path = entry.map_err(|e| failed(from, &e))?.path();
        let target = to.join(path.file_name().ok_or("an entry with no name")?);
        copy_entry(&path, &target)?;
    }
    Ok(())
}

fn copy_entry(path: &Path, target: &Path) -> Result<(), String> {
    if path.is_dir() {
        return copy_dir(path, target);
    }
    std::fs::copy(path, target)
        .map(|_| ())
        .map_err(|e| failed(path, &e))
}

fn failed(path: &Path, error: &std::io::Error) -> String {
    format!("{}: {error}", path.display())
}

#[cfg(test)]
#[path = "serve_publish_tests.rs"]
mod tests;
