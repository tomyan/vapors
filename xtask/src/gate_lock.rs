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

/// touche's gate took 268 to 364 s on the dev machine (2026-10-24 and
/// 25). Up to four implementers may queue, about 24 min, so a gate waits
/// at most 40 min.
pub const WAIT: Duration = Duration::from_mins(40);

/// How often a waiting gate says so.
const SAY_EVERY: Duration = Duration::from_mins(1);

/// The held lock; dropping it frees the lock.
#[derive(Debug)]
pub struct Held {
    _file: File,
}

/// The directory the lock lives in: `~/.cache/factory`.
pub fn dir() -> Result<PathBuf, String> {
    let home =
        std::env::var_os("HOME").ok_or("HOME is unset; the gate lock lives under ~/.cache")?;
    Ok(PathBuf::from(home).join(".cache/factory"))
}

/// Take the lock in `dir` for the gate in `worktree`, waiting up to `wait`.
pub fn acquire(dir: &Path, worktree: &str, wait: Duration) -> Result<Held, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let path = dir.join(LOCK);
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&path)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    let started = Instant::now();
    let mut said = None;
    loop {
        match file.try_lock() {
            Ok(()) => return taken(dir, file, worktree),
            Err(TryLockError::WouldBlock) => {}
            Err(TryLockError::Error(e)) => return Err(format!("{}: {e}", path.display())),
        }
        if started.elapsed() >= wait {
            return Err(format!(
                "the gate lock is still {} after {wait:?}; fix: wait for that gate, or stop it",
                holder(dir)
            ));
        }
        if said.is_none_or(|at: Instant| at.elapsed() >= SAY_EVERY) {
            println!("gate: waiting for the gate lock, {}", holder(dir));
            said = Some(Instant::now());
        }
        thread::sleep(Duration::from_millis(100));
    }
}

fn taken(dir: &Path, file: File, worktree: &str) -> Result<Held, String> {
    let since = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let text = format!("pid {} in {worktree} since {since}\n", std::process::id());
    let path = dir.join(HOLDER);
    std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(Held { _file: file })
}

/// Who holds the lock, from `gate.holder`, with how long ago it started.
fn holder(dir: &Path) -> String {
    let text = std::fs::read_to_string(dir.join(HOLDER)).unwrap_or_default();
    let line = text.trim();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    match line
        .rsplit_once(" since ")
        .and_then(|(who, at)| Some((who, at.parse::<u64>().ok()?)))
    {
        Some((who, at)) => format!("held by {who} for {} s", now.saturating_sub(at)),
        None => format!("held; {} says `{line}`", dir.join(HOLDER).display()),
    }
}

#[cfg(test)]
#[path = "gate_lock_tests.rs"]
mod tests;
