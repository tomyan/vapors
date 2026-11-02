//! Installing a `.zip` download: the archive fetched beside the install
//! directory, its SHA-256 compared, unpacked whole, the binary's SHA-256
//! compared, and only then the tree renamed into place. A mismatch
//! deletes what was fetched; nothing is left in the install directory
//! unless it verified. The binary is never run here.

use std::path::{Path, PathBuf};
use std::process::Command;

use super::{executable, fetch, matches, run};
use crate::downloads::{Download, installed_path, tree_dir};

/// `download` (a tree) installed and verified.
///
/// # Errors
///
/// The fetch, either hash, the unpacking or the move failing.
pub fn ensure(download: &Download, home: &Path) -> Result<PathBuf, String> {
    let dir = tree_dir(download, home);
    let parent = dir.parent().ok_or("the install directory has no parent")?;
    let work = parent.join(format!(
        ".download-{}-{}",
        download.name,
        std::process::id()
    ));
    std::fs::create_dir_all(&work).map_err(|e| format!("{}: {e}", work.display()))?;
    let placed = fetch_and_place(download, &work, &dir);
    let _ = std::fs::remove_dir_all(&work);
    placed.map(|()| installed_path(download, home))
}

fn fetch_and_place(download: &Download, work: &Path, dir: &Path) -> Result<(), String> {
    let archive = work.join("archive.zip");
    fetch(&download.url, &archive)?;
    matches(&archive, &download.sha256, &download.url)?;
    let tree = work.join("tree");
    run(Command::new("unzip")
        .arg("-q")
        .arg(&archive)
        .arg("-d")
        .arg(&tree))
    .map_err(|e| format!("{e}; fix: install unzip"))?;
    let binary = tree.join(&download.member);
    matches(&binary, &download.binary_sha256, &download.member)?;
    executable(&binary)?;
    if dir.exists() {
        std::fs::remove_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    std::fs::rename(&tree, dir).map_err(|e| format!("{} to {}: {e}", tree.display(), dir.display()))
}

#[cfg(test)]
#[path = "download_tree_tests.rs"]
mod tests;
