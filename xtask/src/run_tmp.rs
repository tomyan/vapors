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
//!
//! What it does not cover, so a passing check is not "nothing was left in
//! /tmp": Xvfb's `/tmp/.X11-unix/X<n>` socket and `/tmp/.X<n>-lock`
//! (hard-coded by Xvfb; `xvfb.rs` handles stale ones); Chromium's
//! `/dev/shm` segments; anything under HOME or the XDG directories; and
//! a step that names `/tmp/...` itself. The gate's floor on free inodes
//! is the backstop for those.
//!
//! A run that panics or is killed leaves its directory; the next run's
//! sweep removes it if its pid is not running. Declared limit: "running"
//! is `/proc/<pid>` existing, so a run in another pid namespace sharing
//! /tmp (a container) looks dead and its directory would be swept; and a
//! dead run whose pid was reused stays until that process exits.

use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::steps::Step;

/// The most leftovers a refusal names; the rest are counted.
const NAMED: usize = 20;

/// Set, it keeps what a run left in its TMPDIR (until the next run's
/// sweep), and a failing test's scratch directory (`scratch.rs`), to
/// look at.
pub const KEEP_ENV: &str = "XTASK_KEEP_TMP";

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

    /// Remove the directory; when it held anything, or any of it could
    /// not be read or removed, fail naming all of it. With `keep`, a
    /// directory holding anything is kept to look at instead (until the
    /// next run's sweep).
    ///
    /// # Errors
    /// Entries were left, or could not be read, or the directory could
    /// not be removed (it is then set aside, and the next run is not
    /// blocked by it).
    pub fn finish(self, run: &str, keep: bool) -> Result<(), String> {
        let walked = walk::walk(&self.dir);
        if keep && !(walked.entries.is_empty() && walked.unread.is_empty()) {
            let kept = format!("; kept, since {KEEP_ENV} is set, until the next run");
            return Err(refusal(run, &self.dir, &walked.entries) + &kept);
        }
        let removed = walk::remove_or_set_aside(&self.dir);
        let mut said = Vec::new();
        if !walked.entries.is_empty() {
            said.push(refusal(run, &self.dir, &walked.entries) + "; removed");
        }
        if !walked.unread.is_empty() {
            said.push(format!(
                "{run} could not read all of its TMPDIR: {}",
                walked.unread.join(", ")
            ));
        }
        said.extend(removed.err());
        if said.is_empty() {
            return Ok(());
        }
        Err(said.join("; and "))
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
        "{run} left {count} in its TMPDIR {}: {named}; \
         fix: the test or step that made them removes them (tempfile::TempDir in a test; \
         a TMPDIR of its own for Chromium); reproduce: \
         TMPDIR=$(mktemp -d) cargo nextest run --workspace, then list that directory",
        dir.display()
    )
}

/// A new run's TMPDIR under `base`, once the directories of runs that
/// are not going have been removed, each said on stdout.
///
/// # Errors
/// [`sweep`]'s or [`RunTmp::make`]'s.
pub fn begin(base: &Path) -> Result<RunTmp, String> {
    let who = sweep::Who {
        pid: std::process::id(),
        uid: crate::xvfb_display::current_uid()?,
        is_running: &crate::xvfb_display::process_is_running,
    };
    for line in sweep::sweep(base, &who)? {
        println!("tmp: {line}");
    }
    RunTmp::make(base, who.pid)
}

/// `step` with `tmp` as its TMPDIR.
pub fn with_tmpdir(mut step: Step, tmp: &Path) -> Step {
    step.tmpdir = crate::step::TmpDir::Set(tmp.to_path_buf());
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
    tmp.finish(run, std::env::var_os(KEEP_ENV).is_some())?;
    Ok(code)
}

/// [`around`] under the temp dir, its refusal printed as a failed run.
pub fn around_temp_dir(run: &str, body: impl FnOnce(&Path) -> ExitCode) -> ExitCode {
    around(run, &std::env::temp_dir(), body).unwrap_or_else(|why| {
        println!("FAIL tmp: {why}");
        ExitCode::FAILURE
    })
}

#[path = "run_tmp_walk.rs"]
mod walk;

#[path = "run_tmp_sweep.rs"]
pub mod sweep;

#[cfg(test)]
#[path = "run_tmp_tests.rs"]
mod tests;
