//! At most [`SLOTS`] `cargo xtask ci` runs at once on the machine. Each
//! takes one of the files `ci.slot.<n>` beside the gate lock, exclusive,
//! for its whole run, and writes its owner line into it; the next waits
//! for one to free, saying who holds them. It waits holding `gate.turn`, so a gate that comes after it
//! still goes before the ci runs that come after the gate.
//!
//! Three: each ci run compiles with 4 jobs (`priority.rs`), and three of
//! them fill the dev machine's 12 threads. A ci run beside one or two
//! others took 7 minutes, beside four 19 to 23 (gate logs 270, 271,
//! 274; docs/plans/factory-throughput.md 1.4 and 2.2): past three,
//! builders queue in the kernel and total throughput does not rise.
//!
//! The wait's deadline is the gate lock's 40 minutes, shared with the
//! turn and the lock: a slot frees when one ci run ends, 7 to 23 minutes
//! in the logs above, so a fourth run waits at most about one such run.
//!
//! Declared limit: a slot bounds builders only while its ci process
//! lives. A killed ci frees its slot at once, while the step it was
//! running may still be compiling in its own process group (as for the
//! gate lock, `gate_lock::acquire`). Retired when xtask's steps die
//! with it.

use std::fs::{File, TryLockError};
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

use crate::gate_lock::open;
use crate::lock_owner::Owner;

/// How many ci runs go at once.
pub const SLOTS: usize = 3;

/// How often a waiting ci run says so.
const SAY_EVERY: Duration = Duration::from_mins(1);

/// Take a free slot in `dir`, waiting until `wait` from `started`.
///
/// # Errors
///
/// Every slot is still held at the deadline, naming the holders, or a
/// slot file cannot be opened or locked.
pub fn take(
    dir: &Path,
    worktree: &str,
    (started, wait): (Instant, Duration),
) -> Result<File, String> {
    let mut said: Option<Instant> = None;
    loop {
        if let Some(slot) = free_slot(dir)? {
            return named(slot, dir, worktree);
        }
        if started.elapsed() >= wait {
            return Err(format!(
                "all {SLOTS} ci slots are still taken after {wait:?}, by {}; \
                 fix: wait for one to finish, then rerun `cargo xtask ci`",
                holders(dir)
            ));
        }
        if said.is_none_or(|at| at.elapsed() >= SAY_EVERY) {
            println!(
                "ci: waiting for one of {SLOTS} ci slots, taken by {}",
                holders(dir)
            );
            said = Some(Instant::now());
        }
        thread::sleep(Duration::from_millis(100));
    }
}

/// `slot` with this run's owner line in it, for waiters to name.
fn named(mut slot: File, dir: &Path, worktree: &str) -> Result<File, String> {
    let line = crate::lock_owner::line(worktree);
    let written = slot.set_len(0).and_then(|()| {
        use std::io::{Seek, Write};
        slot.rewind()?;
        slot.write_all(line.as_bytes())
    });
    written.map_err(|e| format!("{}: a ci slot: {e}", dir.display()))?;
    Ok(slot)
}

/// The first slot free now, locked, or none.
fn free_slot(dir: &Path) -> Result<Option<File>, String> {
    for slot in 0..SLOTS {
        let name = format!("ci.slot.{slot}");
        let file = open(dir, &name)?;
        match file.try_lock() {
            Ok(()) => return Ok(Some(file)),
            Err(TryLockError::WouldBlock) => {}
            Err(TryLockError::Error(e)) => {
                return Err(format!("{}: {e}", dir.join(name).display()));
            }
        }
    }
    Ok(None)
}

fn holders(dir: &Path) -> String {
    let holders: Vec<String> = (0..SLOTS).filter_map(|slot| held_by(dir, slot)).collect();
    if holders.is_empty() {
        return "nobody that says so".to_owned();
    }
    holders.join(", ")
}

/// Who holds slot `slot`, if it is held and its owner line is live.
fn held_by(dir: &Path, slot: usize) -> Option<String> {
    let path = dir.join(format!("ci.slot.{slot}"));
    let probe = File::open(&path).ok()?;
    if probe.try_lock_shared().is_ok() {
        return None;
    }
    let owner = Owner::parse(&std::fs::read_to_string(&path).ok()?)?;
    owner.live().then(|| {
        format!(
            "ci in {} (pid {}) for {} s",
            owner.worktree,
            owner.pid,
            owner.held_for()
        )
    })
}

#[cfg(test)]
#[path = "gate_lock_slots_tests.rs"]
mod tests;
