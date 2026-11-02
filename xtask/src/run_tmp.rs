//! Each `ci` and `gate` run's own TMPDIR, `<temp dir>/xtask-run-<pid>`,
//! given to every step and so to what the steps start: tests, Chromium,
//! Xvfb. At the end the run fails if anything is left there, naming it,
//! and the directory is removed either way. /tmp is a tmpfs here, and on
//! 2026-11-02 its 1,048,576 inodes ran out with 23 GB free: test
//! fixtures and Chromium had left a few hundred entries per run.
//!
//! The directory is directly under the temp dir, not under target/:
//! Chromium puts its singleton socket in TMPDIR, and a socket path must
//! fit in 108 bytes. Under a longer TMPDIR, `gpu_devtools` failed with
//! "Socket path too long" (2026-11-02).

use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::steps::Step;

/// The most leftovers a refusal names; the rest are counted.
const NAMED: usize = 20;

/// A run's TMPDIR, made private and empty.
pub struct RunTmp {
    dir: PathBuf,
}

impl RunTmp {
    /// `<base>/xtask-run-<pid>`, refused when it is already there.
    ///
    /// # Errors
    /// It exists or cannot be made, naming it.
    pub fn make(base: &Path, pid: u32) -> Result<Self, String> {
        let dir = base.join(format!("xtask-run-{pid}"));
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&dir)
            .map_err(|e| {
                format!(
                    "{}: {e}; fix: remove it if no run with pid {pid} is going, then rerun",
                    dir.display()
                )
            })?;
        Ok(Self { dir })
    }

    /// The directory, set as each step's TMPDIR.
    pub fn path(&self) -> &Path {
        &self.dir
    }

    /// Remove the directory; when it held anything, fail naming it.
    ///
    /// # Errors
    /// Entries were left (named), or the directory could not be read or
    /// removed.
    pub fn finish(self, run: &str) -> Result<(), String> {
        let left = entries(&self.dir)?;
        std::fs::remove_dir_all(&self.dir)
            .map_err(|e| format!("{}: {e}; fix: remove it", self.dir.display()))?;
        if left.is_empty() {
            return Ok(());
        }
        Err(refusal(run, &self.dir, &left))
    }
}

fn refusal(run: &str, dir: &Path, left: &[String]) -> String {
    let mut named: Vec<String> = left.iter().take(NAMED).cloned().collect();
    if left.len() > NAMED {
        named.push(format!("and {} more", left.len() - NAMED));
    }
    let named = named.join(", ");
    let count = match left.len() {
        1 => "1 entry".to_owned(),
        n => format!("{n} entries"),
    };
    format!(
        "{run} left {count} in its TMPDIR {}, now removed: {named}; \
         fix: the test or step that made them removes them (tempfile::TempDir in a test; \
         a TMPDIR of its own for Chromium); reproduce: \
         TMPDIR=$(mktemp -d) cargo nextest run --workspace, then list that directory",
        dir.display()
    )
}

/// Every entry under `dir`, files and directories, as paths relative to
/// it, sorted.
fn entries(dir: &Path) -> Result<Vec<String>, String> {
    let mut found = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(next) = pending.pop() {
        for entry in read(&next)? {
            let path = entry.path();
            if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                pending.push(path.clone());
            }
            let relative = path.strip_prefix(dir).unwrap_or(&path);
            found.push(relative.display().to_string());
        }
    }
    found.sort();
    Ok(found)
}

fn read(dir: &Path) -> Result<Vec<std::fs::DirEntry>, String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    entries
        .collect::<Result<_, _>>()
        .map_err(|e| format!("{}: {e}", dir.display()))
}

/// A new run's TMPDIR under `base`, once the directories of runs that
/// are not going have been removed, each said on stdout.
///
/// # Errors
/// [`sweep`]'s or [`RunTmp::make`]'s.
pub fn begin(base: &Path) -> Result<RunTmp, String> {
    for line in sweep(base, &crate::xvfb_display::process_is_running)? {
        println!("tmp: {line}");
    }
    RunTmp::make(base, std::process::id())
}

/// `step` with `tmp` as its TMPDIR.
pub fn with_tmpdir(mut step: Step, tmp: &Path) -> Step {
    step.tmpdir = Some(tmp.to_path_buf());
    step
}

/// `body` given a new run TMPDIR under `base`, which is then removed.
///
/// # Errors
/// The TMPDIR could not be made, or held anything once `body` returned
/// (named, as `run` left them).
pub fn around(
    run: &str,
    base: &Path,
    body: impl FnOnce(&Path) -> ExitCode,
) -> Result<ExitCode, String> {
    let tmp = begin(base)?;
    let code = body(tmp.path());
    tmp.finish(run)?;
    Ok(code)
}

/// [`around`] under the temp dir, its refusal printed as a failed run.
pub fn around_temp_dir(run: &str, body: impl FnOnce(&Path) -> ExitCode) -> ExitCode {
    around(run, &std::env::temp_dir(), body).unwrap_or_else(|why| {
        println!("FAIL tmp: {why}");
        ExitCode::FAILURE
    })
}

/// Remove each `xtask-run-<pid>` under `base` whose run is not going (a
/// run that was killed cannot remove its own); one line for each.
///
/// # Errors
/// `base` or such a directory cannot be read or removed.
pub fn sweep(base: &Path, is_running: &dyn Fn(u32) -> bool) -> Result<Vec<String>, String> {
    let mut said = Vec::new();
    for entry in read(base)? {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(pid) = name.strip_prefix("xtask-run-").and_then(|p| p.parse().ok()) else {
            continue;
        };
        if is_running(pid) {
            continue;
        }
        let path = entry.path();
        let count = entries(&path)?.len();
        std::fs::remove_dir_all(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        said.push(format!(
            "removed {} ({count} entries), left by run {pid}, which is not going",
            path.display()
        ));
    }
    Ok(said)
}

#[cfg(test)]
#[path = "run_tmp_tests.rs"]
mod tests;
