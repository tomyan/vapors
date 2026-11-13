//! At most [`SLOTS`] `cargo xtask ci` runs at once on the machine. Each
//! takes one of the files `ci.slot.<n>` beside the gate lock, exclusive,
//! for its whole run; the next waits for one to free, saying who holds
//! them. It waits holding `gate.turn`, so a gate that comes after it
//! still goes before the ci runs that come after the gate.
//!
//! Three: each ci run compiles with 4 jobs (`priority.rs`), and three of
//! them fill the dev machine's 12 threads. A ci run beside one or two
//! others took 7 minutes, beside four 19 to 23 (gate logs 270, 271,
//! 274; docs/plans/factory-throughput.md 1.4 and 2.2): past three,
//! builders queue in the kernel and total throughput does not rise.

use std::fs::{File, TryLockError};
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

use crate::gate_lock::open;
use crate::gate_lock_shared::ci_holders;

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
pub fn take(dir: &Path, (started, wait): (Instant, Duration)) -> Result<File, String> {
    let mut said: Option<Instant> = None;
    loop {
        if let Some(slot) = free_slot(dir)? {
            return Ok(slot);
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
    let holders = ci_holders(dir);
    if holders.is_empty() {
        return "nobody that says so".to_owned();
    }
    holders.join(", ")
}

#[cfg(test)]
#[path = "gate_lock_slots_tests.rs"]
mod tests;
