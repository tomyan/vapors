//! Scratch directories for the target-emptying tests, removed on drop.

use std::os::unix::fs::symlink;
use std::path::PathBuf;

/// A directory of its own for one test, removed when dropped.
pub struct Scratch(pub PathBuf);

impl Scratch {
    pub fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("xtask-targets-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch");
        Self(dir.canonicalize().expect("canonical"))
    }

    /// `rel` under the scratch directory, created as a directory.
    pub fn dir(&self, rel: &str) -> PathBuf {
        let path = self.0.join(rel);
        std::fs::create_dir_all(&path).expect("dir");
        path
    }

    /// `rel` as a repository's worktree: a directory holding `.git`.
    pub fn repo(&self, rel: &str) -> PathBuf {
        self.dir(&format!("{rel}/.git"));
        self.0.join(rel)
    }

    /// A link at `rel` to `to`.
    pub fn link(&self, rel: &str, to: &std::path::Path) -> PathBuf {
        let path = self.0.join(rel);
        symlink(to, &path).expect("link");
        path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
