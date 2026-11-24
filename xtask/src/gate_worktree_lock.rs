//! One gate at a time in a worktree: two would share `target/` and the
//! `GITHUB_OUTPUT` file. The gate holds an exclusive flock on
//! `target/gate/run.lock` for its whole run, with its owner line in the
//! file; a second gate there waits for it, saying whom it waits for, and
//! refuses at the deadline. Gates in other worktrees do not wait on it;
//! they share the machine through `gate_hold.rs`. The lock is the open
//! file's, so it is freed when the gate exits, however it exits.

use std::fs::{File, TryLockError};
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

use crate::lock_owner::Owner;

/// The lock file's name, under `target/gate`.
pub const FILE: &str = "run.lock";

/// The longest a second gate waits: one whole gate, waits and steps
/// (`gate_deadline::LONGEST`, 276 min 50 s).
pub const WAIT: Duration = crate::gate_deadline::LONGEST;

/// How often a waiting gate says so.
const SAY_EVERY: Duration = Duration::from_mins(1);
/// How often a waiting gate tries the lock.
const POLL: Duration = Duration::from_millis(100);

/// Take this worktree's lock, in `target/gate`, for the gate in
/// `worktree`, waiting up to [`WAIT`].
///
/// # Errors
///
/// As [`take`].
pub fn take_here(worktree: &str) -> Result<File, String> {
    let part = "the wait for this worktree's gate lock";
    crate::gate_watchdog::watched(part, WATCHED, || {
        take(Path::new("target/gate"), worktree, WAIT)
    })
}

/// The watchdog's deadline on [`take_here`]: its wait, and the lock
/// file's own open and write (`gate_watchdog::IN_PROCESS`).
pub const WATCHED: Duration = WAIT.saturating_add(crate::gate_watchdog::IN_PROCESS);

/// Take the lock in `dir` (`target/gate`) for the gate in `worktree`,
/// waiting up to `wait`.
///
/// # Errors
///
/// Another gate still holds it at the deadline, naming it and the
/// command to rerun, or the file cannot be made, locked or written.
pub fn take(dir: &Path, worktree: &str, wait: Duration) -> Result<File, String> {
    let file = crate::gate_lock::open(dir, FILE)?;
    let path = dir.join(FILE);
    let started = Instant::now();
    let mut said: Option<Instant> = None;
    while !locked(&file, &path)? {
        if started.elapsed() >= wait {
            return Err(format!(
                "another gate in this worktree still holds {} after {wait:?}: {}; \
                 fix: wait for it to end, then rerun `cargo xtask gate`",
                path.display(),
                holder(&path)
            ));
        }
        if said.is_none_or(|at| at.elapsed() >= SAY_EVERY) {
            println!(
                "gate: waiting for another gate in this worktree, {}",
                holder(&path)
            );
            said = Some(Instant::now());
        }
        thread::sleep(POLL);
    }
    named(file, &path, worktree)
}

/// Whether the lock was free and is now taken.
fn locked(file: &File, path: &Path) -> Result<bool, String> {
    match file.try_lock() {
        Ok(()) => Ok(true),
        Err(TryLockError::WouldBlock) => Ok(false),
        Err(TryLockError::Error(e)) => Err(format!("{}: {e}", path.display())),
    }
}

/// `file` with this gate's owner line in it, for a waiter to name.
fn named(mut file: File, path: &Path, worktree: &str) -> Result<File, String> {
    use std::io::{Seek, Write};
    let line = crate::lock_owner::line(worktree);
    file.set_len(0)
        .and_then(|()| file.rewind())
        .and_then(|()| file.write_all(line.as_bytes()))
        .map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(file)
}

/// The holder its file names: `pid P in W for S s`, or what the file
/// says when it is of another shape.
fn holder(path: &Path) -> String {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    match Owner::parse(&text) {
        Some(owner) => format!(
            "pid {} in {} for {} s",
            owner.pid,
            owner.worktree,
            owner.held_for()
        ),
        None => format!("{} saying `{}`", path.display(), text.trim()),
    }
}

#[cfg(test)]
#[path = "gate_worktree_lock_tests.rs"]
mod tests;
