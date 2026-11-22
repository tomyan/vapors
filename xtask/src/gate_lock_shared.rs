//! A share of the gate lock: `cargo xtask ci`, `cargo xtask test` and a
//! gate's shared steps take it (`share.rs`), so any number of them go
//! together but never beside a gate's exclusive steps, whose GPU and page
//! checks fail on timing under their load (load average 40 to 58,
//! 2026-10-26).
//!
//! An exclusive taker waiting for running shares is not starved by
//! shares asked for after it: both take `gate.turn` first. The exclusive
//! taker keeps the turn while it holds the lock; a share holds it only
//! while it takes its shared lock. Each share names itself in `<who>.<pid>`
//! beside the lock while it holds it, so a waiting taker can say whom it
//! waits for. At most three shares that build (ci runs and gates' shared
//! steps) hold it at once (`gate_lock_slots.rs`); a test run takes no
//! slot.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::gate_lock::{LOCK, Mode, TURN, open, wait_for};
use crate::lock_owner::Owner;

/// A ci run's share of the gate lock; dropping it frees it.
#[derive(Debug)]
pub struct Shared {
    _file: std::fs::File,
    _slot: Option<std::fs::File>,
    marker: PathBuf,
}

impl Drop for Shared {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.marker);
    }
}

/// Whether a share takes one of the ci slots.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    /// It builds: a ci run, a gate's shared step.
    Take,
    /// It does not count against the builders: a test run.
    None,
}

/// Take the lock in `dir` shared for a ci run in `worktree`, waiting up
/// to `wait`: [`acquire_shared_as`] taking a slot, said as `ci`.
///
/// # Errors
///
/// As [`acquire_shared_as`].
#[cfg(test)]
pub fn acquire_shared(dir: &Path, worktree: &str, wait: Duration) -> Result<Shared, String> {
    acquire_shared_as(dir, (worktree, "ci"), wait, Slot::Take)
}

/// Take the lock in `dir` shared for `who` in `worktree`, with a ci slot
/// or not as `slot` says, waiting up to `wait`.
///
/// # Errors
///
/// The lock is still held exclusively, or every ci slot is taken, when
/// `wait` ends, or its files cannot be opened.
pub fn acquire_shared_as(
    dir: &Path,
    (worktree, who): (&str, &str),
    wait: Duration,
    slot: Slot,
) -> Result<Shared, String> {
    let started = Instant::now();
    let turn = open(dir, TURN)?;
    wait_for(dir, (&turn, TURN, Mode::Exclusive), (started, wait), who)?;
    let slot = match slot {
        Slot::Take => Some(crate::gate_lock_slots::take(
            dir,
            (worktree, who),
            (started, wait),
        )?),
        Slot::None => None,
    };
    let file = open(dir, LOCK)?;
    wait_for(dir, (&file, LOCK, Mode::Shared), (started, wait), who)?;
    drop(turn);
    let marker = dir.join(format!("{who}.{}", std::process::id()));
    let line = crate::lock_owner::line(worktree);
    std::fs::write(&marker, line).map_err(|e| format!("{}: {e}", marker.display()))?;
    Ok(Shared {
        _file: file,
        _slot: slot,
        marker,
    })
}

/// Who names a marker, before its pid: `<who>.<pid>`. An xtask built
/// before gates and test runs took shares reads `ci.` markers only, so it
/// names neither while it waits on them; the flock keeps it out either way.
const SHARERS: [&str; 3] = ["ci", "gate", "test"];

/// Each share holding the lock, by its live marker: `<who> in <worktree>
/// (pid N) for S s`. A marker whose process has gone (killed before its
/// drop), or whose pid a later process reuses, is left out.
#[must_use]
pub fn share_holders(dir: &Path) -> Vec<String> {
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
    let (who, pid) = path.file_name()?.to_str()?.split_once('.')?;
    pid.parse::<u32>().ok().filter(|_| SHARERS.contains(&who))?;
    let owner = Owner::parse(&std::fs::read_to_string(path).ok()?)?;
    owner.live().then(|| {
        format!(
            "{who} in {} (pid {}) for {} s",
            owner.worktree,
            owner.pid,
            owner.held_for()
        )
    })
}

#[cfg(test)]
#[path = "gate_lock_shared_tests.rs"]
mod tests;
