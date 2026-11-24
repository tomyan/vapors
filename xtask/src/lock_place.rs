//! Where a run's holds on the gate lock and the quiet lock live, who it
//! is, and how long it waits for each: read from the environment for a
//! real run, built by hand in tests so they lock a directory of their
//! own and wait briefly.

use std::path::PathBuf;
use std::time::Duration;

/// A run's place among the machine's locks.
#[derive(Debug, Clone)]
pub struct Place {
    /// The lock directory, `~/.cache/factory` (`gate_lock::dir`).
    pub dir: PathBuf,
    /// The worktree it runs in, as holders name it.
    pub worktree: String,
    /// The `cargo xtask quiet` that started it, if one did
    /// (`quiet_wait::HOLDER_ENV`).
    pub parent: Option<u32>,
    /// The longest wait for the gate lock and its turn and slots.
    pub lock_wait: Duration,
    /// The longest wait for the quiet lock.
    pub quiet_wait: Duration,
    /// The load wait, run before a share that builds and after an
    /// exclusive hold is taken, named by who waits.
    pub load: fn(&str) -> Result<(), String>,
}

impl Place {
    /// This run's place: `~/.cache/factory`, the current directory, the
    /// quiet parent from the environment, the gate lock's and the quiet
    /// lock's own deadlines, and the machine's load wait.
    ///
    /// # Errors
    ///
    /// HOME is not absolute, or the quiet parent variable is not a pid.
    pub fn from_env() -> Result<Self, String> {
        let parent_env = std::env::var(crate::quiet_wait::HOLDER_ENV).ok();
        Ok(Self {
            dir: crate::gate_lock::dir()?,
            worktree: crate::quiet_wait::worktree(),
            parent: crate::quiet_wait::holder_from_env(parent_env.as_deref())?,
            lock_wait: crate::gate_lock::WAIT,
            quiet_wait: crate::quiet_lock::WAIT,
            load: |who| crate::load_wait::before(who).map(drop),
        })
    }
}

#[cfg(test)]
impl Place {
    /// A place in `dir` for a run in `worktree`, every wait `wait`, no
    /// quiet parent. RELAXED for tests only: the load wait is skipped,
    /// since the machine's load is not what a lock test pins.
    pub fn brief(dir: &std::path::Path, worktree: &str, wait: Duration) -> Self {
        Self {
            dir: dir.to_path_buf(),
            worktree: worktree.to_owned(),
            parent: None,
            lock_wait: wait,
            quiet_wait: wait,
            load: |_| Ok(()),
        }
    }
}
