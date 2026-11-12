//! Scratch directories for tests, removed when dropped.

use std::path::{Path, PathBuf};

/// A test's directory under the temp dir, removed with everything in it
/// when dropped, the test failing or not. /tmp is a RAM-backed tmpfs
/// here, and fixtures that left a directory behind on every run used up
/// its inodes on 2026-11-02.
///
/// Drop cannot fail, so a removal that fails is swallowed here (by
/// tempfile). What makes it loud is the run's leftover check
/// (`run_tmp.rs`): `ci` and `gate` fail naming anything left in their
/// TMPDIR. With `XTASK_KEEP_TMP` set, a test that panics keeps its
/// directory and prints where it is.
pub struct Scratch(Option<tempfile::TempDir>);

impl Scratch {
    fn path(&self) -> &Path {
        self.0
            .as_ref()
            .map_or(Path::new(""), tempfile::TempDir::path)
    }
}

impl std::ops::Deref for Scratch {
    type Target = Path;
    fn deref(&self) -> &Path {
        self.path()
    }
}

impl AsRef<Path> for Scratch {
    fn as_ref(&self) -> &Path {
        self.path()
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let switch = std::env::var_os(crate::run_tmp::KEEP_ENV);
        if !keeps(std::thread::panicking(), switch.is_some()) {
            return;
        }
        if let Some(dir) = self.0.take() {
            eprintln!(
                "kept {} ({} is set)",
                dir.keep().display(),
                crate::run_tmp::KEEP_ENV
            );
        }
    }
}

/// Whether a dropped scratch directory is kept: only for a test that is
/// failing (panicking) with the switch set.
fn keeps(panicking: bool, switch: bool) -> bool {
    panicking && switch
}

/// A new, empty directory named `xtask-<name>-<random>`, so a directory
/// left behind (by a killed run) names the test that made it.
pub fn dir(name: &str) -> Scratch {
    let made = tempfile::Builder::new()
        .prefix(&format!("xtask-{name}-"))
        .tempdir();
    Scratch(Some(made.expect("a scratch directory under the temp dir")))
}

/// A new file named `xtask-<name>-<random>`, removed when dropped.
pub fn file(name: &str) -> tempfile::TempPath {
    let made = tempfile::Builder::new()
        .prefix(&format!("xtask-{name}-"))
        .tempfile();
    made.expect("a scratch file under the temp dir")
        .into_temp_path()
}

/// A lock directory that does not exist yet, for the gate lock's tests,
/// inside a scratch directory removed when the guard drops.
pub struct LockDir {
    _root: Scratch,
    lock: PathBuf,
}

impl std::ops::Deref for LockDir {
    type Target = Path;
    fn deref(&self) -> &Path {
        &self.lock
    }
}

pub fn lock_dir(test: &str) -> LockDir {
    let root = dir(&format!("gate-lock-{test}"));
    let lock = root.join("lock");
    LockDir { _root: root, lock }
}

#[cfg(test)]
#[path = "scratch_tests.rs"]
mod tests;
