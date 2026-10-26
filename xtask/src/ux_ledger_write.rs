//! Writing `spend.jsonl` (`ux_ledger` reads it), and the lock that keeps
//! two reviews from both passing the day's ceiling. Each line is written
//! whole, in one write, and synced, so a crash leaves at most one torn
//! last line, which the reader names.

use std::fs::{File, OpenOptions, TryLockError};
use std::io::Write;
use std::path::Path;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use crate::ux_answer::Usage;

/// How long to wait for another review's hold on the ledger: it holds it
/// from reading the day's spend through its count request (at most
/// curl's 300 s, `ux_call::DEADLINE`) to its `sent` line, so this is that
/// deadline plus 30 s.
pub const LOCK_WAIT: Duration = Duration::from_secs(330);

/// The ledger held: no other review reads the day's spend until it is
/// dropped.
pub struct Held(#[allow(dead_code, reason = "held for its lock, which drop releases")] File);

/// Hold `<dir>/spend.lock`, waiting at most `wait` for another review.
///
/// # Errors
/// The lock cannot be opened, or another review holds it past `wait`.
pub fn hold(dir: &Path, wait: Duration) -> Result<Held, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let path = dir.join("spend.lock");
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&path)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    let until = Instant::now() + wait;
    loop {
        match file.try_lock() {
            Ok(()) => return Ok(Held(file)),
            Err(TryLockError::WouldBlock) if Instant::now() < until => {
                std::thread::sleep(Duration::from_millis(200));
            }
            Err(TryLockError::WouldBlock) => {
                return Err(format!(
                    "another ux-review has held {} for {wait:?}; fix: let it finish, then run again",
                    path.display()
                ));
            }
            Err(TryLockError::Error(e)) => return Err(format!("{}: {e}", path.display())),
        }
    }
}

/// What a line says about one request.
pub struct Entry<'a> {
    pub time: &'a str,
    pub sha: &'a str,
    pub model: &'a str,
    /// Ties a `settled` line to its `sent` line.
    pub id: &'a str,
}

/// An `input_tokens` request: `counted` tokens at `cost`, the input price.
///
/// # Errors
/// The ledger cannot be written.
pub fn count(ledger: &Path, entry: &Entry, counted: u64, cost: f64) -> Result<(), String> {
    let note = "charged at the input price: the documentation does not say counting is free";
    append(
        ledger,
        &json!({ "time": entry.time, "sha": entry.sha, "model": entry.model, "kind": "count", "input": counted, "cost": cost, "costNote": note }),
    )
}

/// A request about to be sent, at its `worst` case.
///
/// # Errors
/// The ledger cannot be written.
pub fn sent(ledger: &Path, entry: &Entry, worst: f64) -> Result<(), String> {
    append(
        ledger,
        &json!({ "time": entry.time, "sha": entry.sha, "model": entry.model, "kind": "sent", "id": entry.id, "cost": worst }),
    )
}

/// What the sent request cost, by its response's usage.
///
/// # Errors
/// The ledger cannot be written.
pub fn settled(ledger: &Path, entry: &Entry, usage: Usage, cost: f64) -> Result<(), String> {
    append(
        ledger,
        &json!({ "time": entry.time, "sha": entry.sha, "model": entry.model, "kind": "settled", "id": entry.id,
        "input": usage.input, "cached": usage.cached, "output": usage.output, "cost": cost }),
    )
}

/// Append `line` whole, in one write, then sync it.
fn append(ledger: &Path, line: &Value) -> Result<(), String> {
    if let Some(dir) = ledger.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(ledger)
        .map_err(|e| format!("{}: {e}", ledger.display()))?;
    let text = format!("{line}\n");
    file.write_all(text.as_bytes())
        .and_then(|()| file.sync_data())
        .map_err(|e| format!("{}: {e}", ledger.display()))
}

#[cfg(test)]
#[path = "ux_ledger_write_tests.rs"]
mod tests;
