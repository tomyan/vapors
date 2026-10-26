//! A scratch lock directory for the gate lock's tests.

/// A test's lock directory, removed when the guard drops: /tmp is a
/// RAM-backed tmpfs here, and every run left one per test behind.
pub struct Scratch(std::path::PathBuf);

impl std::ops::Deref for Scratch {
    type Target = std::path::Path;
    fn deref(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub fn dir(test: &str) -> Scratch {
    let dir = std::env::temp_dir().join(format!("xtask-gate-lock-{test}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    Scratch(dir)
}
