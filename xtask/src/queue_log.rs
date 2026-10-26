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
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|e| format!("{}: {e}", self.path.display()))?;
        writeln!(file, "{line}").map_err(|e| format!("{}: {e}", self.path.display()))
    }
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
