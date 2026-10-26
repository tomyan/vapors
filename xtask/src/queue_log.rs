//! The merge queue's log: one line per decision and action, with its
//! UTC time, appended to the file the foreman reads. The queue messages
//! nobody; this file is how it is heard.

use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// An append-only log file.
pub struct Log {
    /// The file.
    pub path: PathBuf,
}

impl Log {
    /// Append `text` as one line, after the time; also printed.
    ///
    /// # Errors
    ///
    /// The file cannot be opened or written, naming it.
    pub fn line(&self, text: &str) -> Result<(), String> {
        let seconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| format!("the clock: {e}"))?
            .as_secs();
        let line = format!("{} {text}", utc(seconds));
        println!("queue: {line}");
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|e| format!("{}: {e}", self.path.display()))?;
        writeln!(file, "{line}").map_err(|e| format!("{}: {e}", self.path.display()))
    }
}

/// How many days of rotated logs are kept.
pub const KEEP_DAYS: u64 = 7;

/// Rotate `log` on `today` (`YYYY-MM-DD`): a log whose first line is from
/// an earlier day becomes `<log>.<that day>`; rotated logs more than
/// `KEEP_DAYS` before today are deleted.
///
/// # Errors
///
/// A file that cannot be read, renamed or deleted, naming it.
pub fn rotate(log: &std::path::Path, today: &str) -> Result<(), String> {
    let first = match std::fs::read_to_string(log) {
        Ok(text) => text.get(..10).map(str::to_owned),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(format!("{}: {e}", log.display())),
    };
    if let Some(day) = first.filter(|day| day.as_str() < today) {
        let rotated = log.with_file_name(format!("{}.{day}", file_name(log)));
        std::fs::rename(log, &rotated).map_err(|e| format!("{}: {e}", rotated.display()))?;
    }
    delete_old(log, today)
}

fn file_name(log: &std::path::Path) -> String {
    log.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Rotated logs dated more than `KEEP_DAYS` before `today`, deleted.
fn delete_old(log: &std::path::Path, today: &str) -> Result<(), String> {
    let Some(dir) = log.parent() else {
        return Ok(());
    };
    let prefix = format!("{}.", file_name(log));
    let oldest = days_before(today, KEEP_DAYS).ok_or_else(|| format!("{today} is not a date"))?;
    for entry in std::fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .flatten()
    {
        let name = entry.file_name().to_string_lossy().into_owned();
        let dated = name.strip_prefix(&prefix).filter(|d| d.len() == 10);
        if dated.is_some_and(|day| day < oldest.as_str()) {
            std::fs::remove_file(entry.path())
                .map_err(|e| format!("{}: {e}", entry.path().display()))?;
        }
    }
    Ok(())
}

/// The date `days` before `day` (`YYYY-MM-DD`).
fn days_before(day: &str, days: u64) -> Option<String> {
    let mut parts = day.split('-').map(|p| p.parse::<u64>().ok());
    let (year, month, date) = (parts.next()??, parts.next()??, parts.next()??);
    let epoch_days = days_from_civil(year, month, date)?.checked_sub(days)?;
    Some(utc(epoch_days * 86_400)[..10].to_owned())
}

/// A civil date as days since 1970-01-01 (Howard Hinnant's algorithm).
fn days_from_civil(year: u64, month: u64, day: u64) -> Option<u64> {
    let year = if month <= 2 {
        year.checked_sub(1)?
    } else {
        year
    };
    let era = year / 400;
    let yoe = year - era * 400;
    let mp = if month > 2 { month - 3 } else { month + 9 };
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    (era * 146_097 + doe).checked_sub(719_468)
}

/// `seconds` since the epoch as `YYYY-MM-DDTHH:MM:SSZ`.
#[must_use]
pub fn utc(seconds: u64) -> String {
    let (days, rest) = (seconds / 86_400, seconds % 86_400);
    let (year, month, day) = civil(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rest / 3600,
        rest % 3600 / 60,
        rest % 60
    )
}

/// Days since 1970-01-01 as a civil date (Howard Hinnant's algorithm).
fn civil(days: u64) -> (u64, u64, u64) {
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + u64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
#[path = "queue_log_tests.rs"]
mod tests;
