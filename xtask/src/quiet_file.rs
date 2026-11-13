//! The quiet lock's holder file, read whole and claimed atomically.
//! Takers are serialised by an exclusive flock on `quiet.take`, held only
//! while they claim; the holder file is written to a temporary file and
//! linked into place, so a reader never sees half of it.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use super::{FILE, Holder, now_ms};

/// The file takers lock while they claim.
const TAKE: &str = "quiet.take";

/// The holder file in `dir`, ended or not; none when absent.
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
            "{} reads `{}`, not `pid <P> in <worktree> since <s> until <ms> as <form> running <what>`; \
             fix: rm {}",
            path.display(),
            text.trim(),
            path.display()
        )
    })
}

/// Claim the lock for `holder` if no holder file is there or the one
/// there has ended; whether it was claimed.
///
/// # Errors
///
/// A file cannot be written, linked or locked, naming it.
pub fn claim(dir: &Path, holder: &Holder) -> Result<bool, String> {
    let _take = serialised(dir)?;
    if read(dir)?.is_some_and(|there| there.ended(now_ms())) {
        let _ = std::fs::remove_file(dir.join(FILE));
    }
    let temporary = staged(dir, holder)?;
    let linked = std::fs::hard_link(&temporary, dir.join(FILE));
    let _ = std::fs::remove_file(&temporary);
    match linked {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(false),
        Err(e) => Err(format!("{}: {e}", dir.join(FILE).display())),
    }
}

/// How long a taker waits for another to finish claiming. A claim is a
/// read, a write and a link, milliseconds; 10 s is far past that.
const CLAIM_WAIT: std::time::Duration = std::time::Duration::from_secs(10);

/// `quiet.take`, locked, so one taker claims at a time.
fn serialised(dir: &Path) -> Result<std::fs::File, String> {
    let path = dir.join(TAKE);
    let take = crate::gate_lock::open(dir, TAKE)?;
    let started = std::time::Instant::now();
    loop {
        match take.try_lock() {
            Ok(()) => return Ok(take),
            Err(std::fs::TryLockError::Error(e)) => return Err(format!("{}: {e}", path.display())),
            Err(std::fs::TryLockError::WouldBlock) if started.elapsed() >= CLAIM_WAIT => {
                return Err(format!(
                    "{} still locked by another taker after {CLAIM_WAIT:?}; fix: find it with `fuser {}`",
                    path.display(),
                    path.display()
                ));
            }
            Err(std::fs::TryLockError::WouldBlock) => {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
    }
}

/// `holder`'s line in a temporary file of its own beside the holder file.
fn staged(dir: &Path, holder: &Holder) -> Result<PathBuf, String> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let call = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = dir.join(format!("{FILE}.{}.{call}", std::process::id()));
    std::fs::write(&path, holder.line()).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(path)
}

/// Remove the holder file if it names `holder`.
pub fn remove_if(dir: &Path, holder: &Holder) {
    if read(dir).is_ok_and(|there| there.as_ref() == Some(holder)) {
        let _ = std::fs::remove_file(dir.join(FILE));
    }
}

/// Write `holder` whether or not the lock is held, as another process
/// would: for tests.
#[cfg(test)]
pub fn write(dir: &Path, holder: &Holder) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let path = dir.join(FILE);
    std::fs::write(&path, holder.line()).map_err(|e| format!("{}: {e}", path.display()))
}
