//! `cargo xtask ci` takes the gate lock shared, so any number of ci runs
//! go together but never beside a gate, whose GPU and page checks fail
//! on timing under their load (load average 40 to 58, 2026-10-26).
//!
//! A gate waiting for running ci runs is not starved by ci runs started
//! after it: both take `gate.turn` first. The gate keeps the turn for its
//! whole run; a ci run holds it only while it takes its shared lock.
//! Each ci run names itself in `ci.<pid>` beside the lock while it holds
//! it, so a waiting gate can say whom it waits for.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::gate_lock::{LOCK, Mode, TURN, open, wait_for};

/// A ci run's share of the gate lock; dropping it frees it.
#[derive(Debug)]
pub struct Shared {
    _file: std::fs::File,
    marker: PathBuf,
}

impl Drop for Shared {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.marker);
    }
}

/// Take the lock in `dir` shared for a ci run in `worktree`, waiting up
/// to `wait`.
///
/// # Errors
///
/// The lock is still held exclusively (by a gate) when `wait` ends, or
/// its files cannot be opened.
pub fn acquire_shared(dir: &Path, worktree: &str, wait: Duration) -> Result<Shared, String> {
    let started = Instant::now();
    let turn = open(dir, TURN)?;
    wait_for(dir, (&turn, Mode::Exclusive), (started, wait), "ci")?;
    let file = open(dir, LOCK)?;
    wait_for(dir, (&file, Mode::Shared), (started, wait), "ci")?;
    drop(turn);
    let marker = dir.join(format!("{MARKER}{}", std::process::id()));
    std::fs::write(&marker, worktree).map_err(|e| format!("{}: {e}", marker.display()))?;
    Ok(Shared {
        _file: file,
        marker,
    })
}

/// A ci run's marker's name, before its pid.
const MARKER: &str = "ci.";

/// Each ci run holding the lock, by its live marker: `ci in <worktree>
/// (pid N)`. A marker whose process has gone (killed before its drop) is
/// left out.
#[must_use]
pub fn ci_holders(dir: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut holders: Vec<String> = entries
        .flatten()
        .filter_map(|entry| live_marker(&entry.path()))
        .collect();
    holders.sort();
    holders
}

fn live_marker(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_str()?;
    let pid = name.strip_prefix(MARKER)?;
    if !Path::new("/proc").join(pid).exists() {
        return None;
    }
    let worktree = std::fs::read_to_string(path).ok()?;
    Some(format!("ci in {} (pid {pid})", worktree.trim()))
}

#[cfg(test)]
#[path = "gate_lock_shared_tests.rs"]
mod tests;
