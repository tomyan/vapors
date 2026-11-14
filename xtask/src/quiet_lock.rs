//! The quiet lock: one holder at a time keeps xtask's builds and test
//! runs from starting, so a timed run gets the machine to itself. It is
//! a file, `~/.cache/factory/quiet.holder`, naming who holds it and until
//! when, and in which form; `cargo xtask ci`, `test` and `gate` wait while
//! it is held (`quiet_wait.rs`), saying who holds it and when it expires.
//! A hold ends at its deadline, which is hard: a forgotten hold blocks
//! nobody past it. A gate's hold and a `quiet <minutes> -- <command>`
//! hold also end when their process does, however it ends (its pid gone
//! or reused, as `lock_owner` reads it); a `quiet hold` outlives its
//! process by design. The gate takes it for its run (`gate_floor.rs`);
//! `cargo xtask quiet` takes it by hand (`quiet_command.rs`).
//!
//! Declared limits: plain `cargo` outside xtask does not wait, and nor
//! does an xtask built before this file, in another worktree or in
//! touche, until that worktree has rebased; a waiter polls, so two
//! holders waiting on the same expiry are not served in turn.

use std::path::Path;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[path = "quiet_holder.rs"]
mod holder;
pub use holder::{Form, Holder};
#[path = "quiet_file.rs"]
mod file;
pub use file::read;
#[cfg(test)]
use file::write;

/// The holder file's name, beside the gate lock.
pub const FILE: &str = "quiet.holder";

/// The longest hold: gate 270's 2,885 s from start to end (lock wait,
/// load wait and steps), the longest of the 65 gates in
/// docs/plans/factory-throughput.md, rounded up to the hour. The gate
/// holds the lock this long; a hold by hand may not be longer.
pub const LONGEST: Duration = Duration::from_mins(60);

/// A waiter's own deadline: the longest hold and a minute, so only a
/// holder file the deadline check misread can reach it.
pub const WAIT: Duration = Duration::from_mins(61);

/// How often a waiter says so.
const SAY_EVERY: Duration = Duration::from_mins(1);
/// How often a waiter reads the holder file.
const POLL: Duration = Duration::from_millis(100);

/// The held lock; dropping it removes the holder file if it is still ours.
#[derive(Debug)]
pub struct Quiet {
    dir: std::path::PathBuf,
    holder: Holder,
}

impl Quiet {
    /// Keep the lock after this process ends, until its deadline or
    /// `cargo xtask quiet release`.
    pub fn keep(self) {
        std::mem::forget(self);
    }

    /// Whether the holder file still names this hold, before its deadline.
    pub fn still_held(&self) -> bool {
        read(&self.dir).is_ok_and(|now| now.as_ref() == Some(&self.holder))
            && self.holder.left(now_ms()).is_some()
    }
}

impl Drop for Quiet {
    fn drop(&mut self) {
        file::remove_if(&self.dir, &self.holder);
    }
}

/// Take the lock in `dir` for `holder`, waiting for a current holder's
/// deadline first, never past `wait` ([`WAIT`] outside tests).
///
/// # Errors
///
/// The hold is longer than [`LONGEST`], the holder file is of another
/// shape or still held when `wait` ends, or it cannot be written.
pub fn take(dir: &Path, holder: Holder, wait: Duration) -> Result<Quiet, String> {
    if holder.left(now_ms()).is_some_and(|left| left > LONGEST) {
        return Err(format!(
            "a quiet hold longer than {} minutes; fix: ask for {} or fewer",
            LONGEST.as_secs() / 60,
            LONGEST.as_secs() / 60
        ));
    }
    let started = std::time::Instant::now();
    loop {
        let left = wait.saturating_sub(started.elapsed());
        wait_clear(dir, (&holder.what, &|_| false), left, &mut |line| {
            println!("{line}");
        })?;
        if file::claim(dir, &holder)? {
            return Ok(Quiet {
                dir: dir.to_path_buf(),
                holder,
            });
        }
    }
}

/// Wait while a holder that is not `mine` holds the lock in `dir`, calling
/// `say` each minute, `who` naming the waiter: until the holder's
/// deadline, and never past `wait`.
///
/// # Errors
///
/// The holder file is of another shape, names a deadline past
/// [`LONGEST`] from now, or is still held when `wait` ends.
pub fn wait_clear(
    dir: &Path,
    (who, mine): (&str, &dyn Fn(&Holder) -> bool),
    wait: Duration,
    say: &mut dyn FnMut(&str),
) -> Result<(), String> {
    let started = std::time::Instant::now();
    let mut said: Option<std::time::Instant> = None;
    while let Some(holder) = current(dir)? {
        if mine(&holder) {
            return Ok(());
        }
        if started.elapsed() >= wait {
            return Err(format!(
                "the quiet lock is still held by {} after {wait:?}; fix: rm {}",
                holder.describe(now_ms()),
                dir.join(FILE).display()
            ));
        }
        if said.is_none_or(|at| at.elapsed() >= SAY_EVERY) {
            say(&format!(
                "{who}: waiting for the quiet lock, held by {}",
                holder.describe(now_ms())
            ));
            said = Some(std::time::Instant::now());
        }
        thread::sleep(POLL);
    }
    Ok(())
}

/// The holder that has not ended, or none.
///
/// # Errors
///
/// As [`read`], or its deadline is past [`LONGEST`] from now.
pub fn current(dir: &Path) -> Result<Option<Holder>, String> {
    let Some(holder) = read(dir)? else {
        return Ok(None);
    };
    if holder.ended(now_ms())? {
        return Ok(None);
    }
    match holder.left(now_ms()) {
        None => Ok(None),
        Some(left) if left > LONGEST + Duration::from_secs(60) => Err(format!(
            "{} names a deadline {} s away, past the longest hold of {} s; fix: rm {}",
            dir.join(FILE).display(),
            left.as_secs(),
            LONGEST.as_secs(),
            dir.join(FILE).display()
        )),
        Some(_) => Ok(Some(holder)),
    }
}

/// End `holder`'s hold in `dir` if the file still names it (`quiet
/// release`), under the takers' lock.
pub fn release(dir: &Path, holder: &Holder) {
    file::remove_if(dir, holder);
}

/// Now, in unix milliseconds.
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

#[cfg(test)]
#[path = "quiet_lock_tests.rs"]
mod tests;
