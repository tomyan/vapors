//! The cap on a worktree's build directory: every build xtask starts
//! (each step, as with the compilation cache, `sccache.rs`) is refused
//! while `target/` holds more than `[targets] max_gb` in
//! `.config/factory.toml`, naming the size and the command that empties
//! it. On 2026-11-15 one worktree's target passed 80 GB twice and the
//! disk reached 95%.
//!
//! The size is measured cheaply: the files directly in each
//! `target/<profile>/deps` and `target/<triple>/<profile>/deps`, one
//! directory listing each with no walk below, summed as allocated
//! blocks (as `du` counts). Measured 2026-11-16 on six of the dev box's
//! targets, `deps` held 89% to 96% of what `du -sb` counted (98.3 of
//! 111.0 GB, 84.3 of 90.0, 47.1 of 50.8, 31.5 of 32.7, 1.3 of 1.4,
//! 0.7 of 0.9), read in 2 to 34 ms where `du` took 9 to 400 ms warm and
//! more cold. So the cap fires at `max_gb` of build products, with the
//! whole directory then at most about an eighth larger.

use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

const GB: u64 = 1_000_000_000;

/// Refuse when the workspace's `target/` holds more than the configured
/// cap; a repository that sets none, or a directory with no settings
/// file, is not capped, said once per run.
///
/// # Errors
/// Over the cap, the cap not a whole number, or a `deps` directory that
/// could not be read, each naming the fix.
pub fn check() -> Result<(), String> {
    // xtask's own unit tests run steps from the xtask crate, which has
    // no settings file; a workspace always does
    if !Path::new(".config/factory.toml").exists() {
        uncapped_once("no .config/factory.toml in the current directory");
        return Ok(());
    }
    let config = crate::config::Config::load().map_err(|e| e.to_string())?;
    let Some(max_gb) = cap(config.get("targets.max_gb"))? else {
        uncapped_once("no [targets] max_gb in .config/factory.toml");
        return Ok(());
    };
    let target = crate::gate::floor::target_dir(Path::new("."))?;
    if target == Path::new(".") {
        return Ok(()); // no target/ yet: nothing built
    }
    let held = deps_bytes(&target)?;
    over(&target, held, max_gb)
}

/// `[targets] max_gb` as a number of GB; `None` when unset.
///
/// # Errors
/// Set to anything but a finite number above 0.
pub fn cap(value: Option<&str>) -> Result<Option<f64>, String> {
    let Some(text) = value else {
        return Ok(None);
    };
    match text.parse::<f64>() {
        Ok(gb) if gb.is_finite() && gb > 0.0 => Ok(Some(gb)),
        _ => Err(format!(
            ".config/factory.toml: [targets] max_gb = \"{text}\" is not a number of GB above 0; fix: set it so, such as \"80\""
        )),
    }
}

/// Bytes allocated to the files directly in `target`'s `deps`
/// directories, one and two levels down.
///
/// # Errors
/// A directory that exists but could not be listed, or an entry whose
/// size could not be read for a reason other than its removal meanwhile.
pub fn deps_bytes(target: &Path) -> Result<u64, String> {
    let mut total = 0;
    for deps in deps_dirs(target)? {
        total += listed_bytes(&deps)?;
    }
    Ok(total)
}

/// `target/*/deps` and `target/*/*/deps` that exist.
fn deps_dirs(target: &Path) -> Result<Vec<PathBuf>, String> {
    let mut found = Vec::new();
    for level_one in subdirs(target)? {
        found.push(level_one.join("deps"));
        found.extend(subdirs(&level_one)?.into_iter().map(|dir| dir.join("deps")));
    }
    found.retain(|dir| dir.is_dir());
    Ok(found)
}

fn subdirs(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let entries = std::fs::read_dir(dir).map_err(|e| unreadable(dir, &e))?;
    Ok(entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir() && !path.is_symlink())
        .collect())
}

/// The allocated bytes of the entries directly in `dir`.
fn listed_bytes(dir: &Path) -> Result<u64, String> {
    let mut total = 0;
    for entry in std::fs::read_dir(dir).map_err(|e| unreadable(dir, &e))? {
        let entry = entry.map_err(|e| unreadable(dir, &e))?;
        match entry.metadata() {
            Ok(meta) => total += meta.blocks() * 512,
            // a build removed it meanwhile: it holds nothing now
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(unreadable(&entry.path(), &e)),
        }
    }
    Ok(total)
}

fn unreadable(path: &Path, error: &std::io::Error) -> String {
    format!(
        "{}: {error}, so the build directory's size could not be measured; fix: check its permissions, or empty it",
        path.display()
    )
}

/// Refuse when `held` bytes pass `max_gb`, naming the exact command.
///
/// # Errors
/// Over the cap.
#[allow(
    clippy::cast_precision_loss,
    reason = "bytes of a disk, far inside f64's exact range"
)]
pub fn over(target: &Path, held: u64, max_gb: f64) -> Result<(), String> {
    if held as f64 <= max_gb * GB as f64 {
        return Ok(());
    }
    let real = target
        .canonicalize()
        .unwrap_or_else(|_| target.to_path_buf());
    Err(format!(
        "{} holds {} GB of build products (target/*/deps), over [targets] max_gb = {max_gb} in .config/factory.toml; \
         fix: `find {} -mindepth 1 -maxdepth 1 -exec rm -rf {{}} +` (empties it and keeps the directory, so the \
         worktree's target link stays valid), then rerun",
        target.display(),
        held / GB,
        real.display()
    ))
}

fn uncapped_once(why: &str) {
    static SAID: std::sync::Once = std::sync::Once::new();
    SAID.call_once(|| println!("targets: {why}; the build directory is not capped"));
}

#[cfg(test)]
#[path = "target_cap_tests.rs"]
mod tests;
