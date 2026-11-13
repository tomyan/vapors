//! The quiet lock: one holder at a time keeps xtask's builds and test
//! runs from starting, so a timed run gets the machine to itself. It is
//! a file, `~/.cache/factory/quiet.holder`, naming who holds it and until
//! when; `cargo xtask ci`, `test` and `gate` wait while it is held, saying
//! who holds it and when it expires. The deadline is hard: a holder that
//! crashed or forgot to release blocks nobody past it, and a waiter goes
//! on at it. The gate takes it for its run (`gate_floor.rs`);
//! `cargo xtask quiet` takes it by hand (`quiet_command.rs`).
//!
//! Declared limits: plain `cargo` outside xtask does not wait; a waiter
//! polls, so two holders waiting on the same expiry are not served in
//! turn.

use std::io::Write;
use std::path::Path;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[path = "quiet_holder.rs"]
mod holder;
pub use holder::Holder;

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
}

impl Drop for Quiet {
    fn drop(&mut self) {
        if read(&self.dir).is_ok_and(|now| now.as_ref() == Some(&self.holder)) {
            let _ = std::fs::remove_file(self.dir.join(FILE));
        }
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
        wait_clear(dir, (&holder.what, None), left, &mut |line| {
            println!("{line}");
        })?;
        if create(dir, &holder)? {
            return Ok(Quiet {
                dir: dir.to_path_buf(),
                holder,
            });
        }
    }
}

/// Wait while someone other than `me` holds the lock in `dir`, calling
/// `say` each minute, `who` naming the waiter: until the holder's
/// deadline, and never past `wait`.
///
/// # Errors
///
/// The holder file is of another shape, names a deadline past
/// [`LONGEST`] from now, or is still held when `wait` ends.
pub fn wait_clear(
    dir: &Path,
    (who, me): (&str, Option<u32>),
    wait: Duration,
    say: &mut dyn FnMut(&str),
) -> Result<(), String> {
    let started = std::time::Instant::now();
    let mut said: Option<std::time::Instant> = None;
    while let Some(holder) = current(dir)? {
        if Some(holder.pid) == me {
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

/// The holder before its deadline, or none.
fn current(dir: &Path) -> Result<Option<Holder>, String> {
    let Some(holder) = read(dir)? else {
        return Ok(None);
    };
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

/// The holder file in `dir`, expired or not; none when absent.
///
/// # Errors
///
/// It is unreadable or of another shape, naming it and the fix.
pub fn read(dir: &Path) -> Result<Option<Holder>, String> {
    let path = dir.join(FILE);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("{}: {e}", path.display())),
    };
    Holder::parse(&text).map(Some).ok_or_else(|| {
        format!(
            "{} reads `{}`, not `pid <P> in <worktree> until <unix ms> running <what>`; fix: rm {}",
            path.display(),
            text.trim(),
            path.display()
        )
    })
}

/// Write `holder` whether or not the lock is held, as another process
/// would: for tests.
#[cfg(test)]
fn write(dir: &Path, holder: &Holder) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let path = dir.join(FILE);
    std::fs::write(&path, holder.line()).map_err(|e| format!("{}: {e}", path.display()))
}

/// Write `holder` if no file is there, or the one there is past its
/// deadline; whether it was written.
fn create(dir: &Path, holder: &Holder) -> Result<bool, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let path = dir.join(FILE);
    if read(dir)?.is_some_and(|there| there.left(now_ms()).is_none()) {
        let _ = std::fs::remove_file(&path);
    }
    let opened = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path);
    match opened {
        Ok(mut file) => file
            .write_all(holder.line().as_bytes())
            .map(|()| true)
            .map_err(|e| format!("{}: {e}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(false),
        Err(e) => Err(format!("{}: {e}", path.display())),
    }
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
