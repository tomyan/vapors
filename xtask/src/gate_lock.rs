//! One gate at a time on the machine. The gate holds an exclusive lock on
//! `~/.cache/factory/gate.lock` for its whole run, and writes who holds it
//! to `gate.holder` beside it. The lock is the open file's, so it is freed
//! when the gate exits, however it exits. A second gate waits, printing
//! who holds the lock, and fails once its deadline passes.

use std::fs::{File, OpenOptions, TryLockError};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// The lock file's name.
pub const LOCK: &str = "gate.lock";
/// The file naming the gate that holds the lock.
pub const HOLDER: &str = "gate.holder";
/// The turnstile a gate holds for its whole run and a ci run only while
/// it takes its share (`gate_lock_shared.rs`), so a waiting gate goes
/// before ci runs that come after it.
pub const TURN: &str = "gate.turn";

/// touche's gate took 268 to 364 s on the dev machine (2026-10-24 and
/// 25). Up to four implementers may queue, about 24 min, so a gate waits
/// at most 40 min.
pub const WAIT: Duration = Duration::from_mins(40);

/// How often a waiting gate says so.
const SAY_EVERY: Duration = Duration::from_mins(1);

/// The held lock; dropping it frees the lock and its holder file.
#[derive(Debug)]
pub struct Held {
    _file: File,
    _turn: File,
    holder: PathBuf,
}

impl Drop for Held {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.holder);
    }
}

/// How a lock file is taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// By one gate alone.
    Exclusive,
    /// By any number of ci runs together.
    Shared,
}

/// The directory the lock lives in: `~/.cache/factory`.
pub fn dir() -> Result<PathBuf, String> {
    dir_in(std::env::var_os("HOME"))
}

/// `<home>/.cache/factory`; an unset, empty or relative home is refused,
/// since each worktree would then lock a directory of its own.
pub fn dir_in(home: Option<std::ffi::OsString>) -> Result<PathBuf, String> {
    let home = PathBuf::from(home.unwrap_or_default());
    if !home.is_absolute() {
        return Err(format!(
            "HOME is `{}`, not an absolute path; the gate lock lives under ~/.cache",
            home.display()
        ));
    }
    Ok(home.join(".cache/factory"))
}

/// Take the lock in `dir` for the gate in `worktree`, waiting up to `wait`.
///
/// Declared limits: waiting gates are not served in turn, since each
/// retries every 100 ms, and a gate killed mid-step frees the lock while
/// that step's process group may still run.
pub fn acquire(dir: &Path, worktree: &str, wait: Duration) -> Result<Held, String> {
    let started = Instant::now();
    let turn = open(dir, TURN)?;
    wait_for(dir, (&turn, Mode::Exclusive), (started, wait), "gate")?;
    let file = open(dir, LOCK)?;
    wait_for(dir, (&file, Mode::Exclusive), (started, wait), "gate")?;
    taken(dir, file, turn, worktree)
}

/// Wait until `file` is locked in `mode`, saying whom it waits for each
/// minute, until `wait` from `started` has passed.
///
/// # Errors
///
/// The deadline passed, naming who holds the lock, or the lock failed.
pub fn wait_for(
    dir: &Path,
    (file, mode): (&File, Mode),
    (started, wait): (Instant, Duration),
    who: &str,
) -> Result<(), String> {
    let mut said: Option<Instant> = None;
    while !try_take(file, dir, mode)? {
        if started.elapsed() >= wait {
            return Err(format!(
                "the gate lock is still {} after {wait:?}; fix: wait for it, or stop it",
                holder(dir)
            ));
        }
        if said.is_none_or(|at| at.elapsed() >= SAY_EVERY) {
            println!("{who}: waiting for the gate lock, {}", holder(dir));
            said = Some(Instant::now());
        }
        thread::sleep(Duration::from_millis(100));
    }
    Ok(())
}

/// The lock file `name` in `dir`, made when missing.
///
/// # Errors
///
/// The directory or file cannot be made or opened, naming it.
pub fn open(dir: &Path, name: &str) -> Result<File, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let path = dir.join(name);
    OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&path)
        .map_err(|e| format!("{}: {e}", path.display()))
}

/// Whether the lock was free and is now taken.
fn try_take(file: &File, dir: &Path, mode: Mode) -> Result<bool, String> {
    let tried = match mode {
        Mode::Exclusive => file.try_lock(),
        Mode::Shared => file.try_lock_shared(),
    };
    match tried {
        Ok(()) => Ok(true),
        Err(TryLockError::WouldBlock) => Ok(false),
        Err(TryLockError::Error(e)) => Err(format!("{}: {e}", dir.join(LOCK).display())),
    }
}

/// Record who holds the lock, written whole: to a temporary file, then
/// renamed over `gate.holder`, so a waiter never reads half of it.
fn taken(dir: &Path, file: File, turn: File, worktree: &str) -> Result<Held, String> {
    let since = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let text = format!("pid {} in {worktree} since {since}\n", std::process::id());
    let (path, temporary) = (
        dir.join(HOLDER),
        dir.join(format!("{HOLDER}.{}", std::process::id())),
    );
    std::fs::write(&temporary, text).map_err(|e| format!("{}: {e}", temporary.display()))?;
    std::fs::rename(&temporary, &path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(Held {
        _file: file,
        _turn: turn,
        holder: path,
    })
}

/// Who holds the lock: the gate named in `gate.holder`, with how long
/// ago it started, and each ci run with a live `ci.<pid>` marker.
fn holder(dir: &Path) -> String {
    let mut holders: Vec<String> = gate_holder(dir).into_iter().collect();
    holders.extend(crate::gate_lock_shared::ci_holders(dir));
    if holders.is_empty() {
        return "held, by nobody that says so".to_owned();
    }
    format!("held by {}", holders.join(", "))
}

fn gate_holder(dir: &Path) -> Option<String> {
    let text = std::fs::read_to_string(dir.join(HOLDER)).ok()?;
    let line = text.trim();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    match line
        .rsplit_once(" since ")
        .and_then(|(who, at)| Some((who, at.parse::<u64>().ok()?)))
    {
        Some((who, at)) => Some(format!("{who} for {} s", now.saturating_sub(at))),
        None => Some(format!("{} saying `{line}`", dir.join(HOLDER).display())),
    }
}

#[cfg(test)]
#[path = "gate_lock_tests.rs"]
mod tests;
