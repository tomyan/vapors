//! Scratch directories for tests, removed when dropped.

use std::path::{Path, PathBuf};

/// A test's directory under the temp dir, removed with everything in it
/// when dropped, the test failing or not. /tmp is a RAM-backed tmpfs
/// here, and fixtures that left a directory behind on every run used up
/// its inodes on 2026-11-02.
pub struct Scratch(tempfile::TempDir);

impl std::ops::Deref for Scratch {
    type Target = Path;
    fn deref(&self) -> &Path {
        self.0.path()
    }
}

impl AsRef<Path> for Scratch {
    fn as_ref(&self) -> &Path {
        self.0.path()
    }
}

/// A new, empty directory named `xtask-<name>-<random>`, so a directory
/// left behind (by a killed run) names the test that made it.
pub fn dir(name: &str) -> Scratch {
    let made = tempfile::Builder::new()
        .prefix(&format!("xtask-{name}-"))
        .tempdir();
    Scratch(made.expect("a scratch directory under the temp dir"))
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
