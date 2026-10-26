//! Installing and checking a `[downloads]` tool (`downloads.rs`). The
//! archive is fetched into a temporary directory beside the install path,
//! its SHA-256 compared, the binary extracted and its SHA-256 compared,
//! and only then made executable and renamed into place. A mismatch
//! deletes what was fetched and names both hashes. The binary is never
//! run here: what it is comes from its hash, not from `--version`.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use crate::deadline::{Finished, run_with_deadline};
use crate::downloads::{Download, installed_path};

/// The 2.44.1 archive (4.6 MB) downloaded in under 2 s on the dev box
/// (2026-10-26); 120 s allows a slow link without hanging setup.
const FETCH_DEADLINE: Duration = Duration::from_secs(120);

/// The installed binary, hashed and matching its pin.
///
/// # Errors
///
/// Not installed, or another binary at the path, naming the fix.
pub fn verified(download: &Download, home: &Path) -> Result<PathBuf, String> {
    let path = installed_path(download, home);
    if !path.is_file() {
        return Err(format!(
            "{} is not installed; fix: cargo xtask setup",
            path.display()
        ));
    }
    let found = sha256(&path)?;
    if found != download.binary_sha256 {
        return Err(format!(
            "{} has SHA-256 {found}, not the pinned {}; fix: delete it and run cargo xtask setup",
            path.display(),
            download.binary_sha256
        ));
    }
    Ok(path)
}

/// `download` installed and verified, fetching it when it is not.
///
/// # Errors
///
/// The fetch, either hash, the extraction or the move failing; nothing is
/// left at the install path unless it verified.
pub fn ensure(download: &Download, home: &Path) -> Result<PathBuf, String> {
    if let Ok(path) = verified(download, home) {
        return Ok(path);
    }
    let path = installed_path(download, home);
    let bin = path.parent().ok_or("the install path has no directory")?;
    let work = bin.join(format!(
        ".download-{}-{}",
        download.name,
        std::process::id()
    ));
    std::fs::create_dir_all(&work).map_err(|e| format!("{}: {e}", work.display()))?;
    let installed = fetch_and_place(download, &work, &path);
    let _ = std::fs::remove_dir_all(&work);
    installed.map(|()| path)
}

fn fetch_and_place(download: &Download, work: &Path, path: &Path) -> Result<(), String> {
    let archive = work.join("archive.tar.gz");
    fetch(&download.url, &archive)?;
    matches(&archive, &download.sha256, &download.url)?;
    run(Command::new("tar")
        .arg("-xzf")
        .arg(&archive)
        .arg("-C")
        .arg(work)
        .arg(&download.member))?;
    let binary = work.join(&download.member);
    matches(&binary, &download.binary_sha256, &download.member)?;
    executable(&binary)?;
    std::fs::rename(&binary, path)
        .map_err(|e| format!("{} to {}: {e}", binary.display(), path.display()))
}

fn fetch(url: &str, to: &Path) -> Result<(), String> {
    let mut command = Command::new("curl");
    command.args(["-sSfL", "-o"]).arg(to).arg(url);
    match run_with_deadline(command, FETCH_DEADLINE)? {
        Finished::Exited { success: true, .. } => Ok(()),
        Finished::Exited { output, .. } => Err(format!("curl {url} failed: {output}")),
        Finished::TimedOut { .. } => Err(format!(
            "{url} did not download within {} s",
            FETCH_DEADLINE.as_secs()
        )),
    }
}

fn matches(file: &Path, pinned: &str, what: &str) -> Result<(), String> {
    let found = sha256(file)?;
    if found == pinned {
        return Ok(());
    }
    Err(format!(
        "{what} has SHA-256 {found}, not the pinned {pinned}; deleted, nothing installed; fix: check the release, then bump [downloads] in .config/tools.toml in a pull request"
    ))
}

/// The file's SHA-256 as `sha256sum` prints it.
///
/// # Errors
///
/// `sha256sum` missing or failing.
pub fn sha256(file: &Path) -> Result<String, String> {
    let output = Command::new("sha256sum")
        .arg(file)
        .output()
        .map_err(|e| format!("sha256sum: {e}; fix: install coreutils"))?;
    let text = String::from_utf8_lossy(&output.stdout);
    match text.split_whitespace().next() {
        Some(hash) if output.status.success() && hash.len() == 64 => Ok(hash.to_owned()),
        _ => Err(format!(
            "sha256sum {} failed: {}",
            file.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        )),
    }
}

fn run(command: &mut Command) -> Result<(), String> {
    let output = command.output().map_err(|e| format!("{command:?}: {e}"))?;
    if output.status.success() {
        return Ok(());
    }
    Err(format!(
        "{command:?} failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    ))
}

#[cfg(unix)]
fn executable(file: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(file, std::fs::Permissions::from_mode(0o755))
        .map_err(|e| format!("{}: {e}", file.display()))
}

#[cfg(not(unix))]
fn executable(_file: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
#[path = "download_install_tests.rs"]
mod tests;
