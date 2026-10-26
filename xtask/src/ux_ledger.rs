//! `spend.jsonl` in the report directory: one line per live review, with
//! its time (UTC), commit, model, tokens and cost. Before a review is
//! sent, the day's spend plus its worst case must stay under
//! `ux.daily_cost_usd`.

use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use crate::ux_answer::Usage;

/// The ledger's file name in the report directory.
pub const FILE: &str = "spend.jsonl";

/// `now` as `YYYY-MM-DDTHH:MM:SSZ`.
pub fn utc(now: SystemTime) -> String {
    let secs = now.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let (days, rest) = (secs / 86_400, secs % 86_400);
    let (year, month, day) = civil(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rest / 3600,
        rest % 3600 / 60,
        rest % 60
    )
}

/// The calendar date `days` after 1970-01-01 (Howard Hinnant's
/// `civil_from_days`, for days since the epoch).
fn civil(days: u64) -> (u64, u64, u64) {
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + u64::from(month <= 2), month, day)
}

/// The cost of every run recorded on `date` (`YYYY-MM-DD`); none when
/// the ledger does not exist yet.
///
/// # Errors
/// An unreadable ledger, or a line that is not a run, naming the line.
pub fn spent_on(ledger: &Path, date: &str) -> Result<f64, String> {
    let text = match std::fs::read_to_string(ledger) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(0.0),
        Err(e) => return Err(format!("{}: {e}", ledger.display())),
    };
    let mut total = 0.0;
    for (index, line) in text.lines().enumerate() {
        let run: Value = serde_json::from_str(line)
            .map_err(|e| format!("{}:{}: not a run: {e}", ledger.display(), index + 1))?;
        if is_count(&run) {
            continue;
        }
        let (Some(time), Some(cost)) = (run["time"].as_str(), run["cost"].as_f64()) else {
            return Err(format!(
                "{}:{}: a run with no `time` or `cost`",
                ledger.display(),
                index + 1
            ));
        };
        if time.starts_with(date) {
            total += cost;
        }
    }
    Ok(total)
}

/// The cost a count request's line records: its response carries no
/// usage and no charge.
pub const COUNT_COST: &str = "unknown, not billed per the response";

/// A count request's line, which adds nothing to the day's spend: its
/// cost is not known (`COUNT_COST`).
fn is_count(run: &Value) -> bool {
    run["kind"] == "count" && run["cost"].is_null() && run["costNote"] == COUNT_COST
}

/// Append a count request's line: its time, commit, model and the input
/// tokens it counted, with its cost recorded as not known.
///
/// # Errors
/// The ledger cannot be written, naming it.
pub fn record_count(
    ledger: &Path,
    time: &str,
    sha: &str,
    model: &str,
    counted: u64,
) -> Result<(), String> {
    let line = json!({ "time": time, "sha": sha, "model": model, "kind": "count", "input": counted, "cost": null, "costNote": COUNT_COST });
    append(ledger, &line)
}

/// Refuse a run whose worst case would take the day's spend over `ceiling`.
///
/// # Errors
/// Over the ceiling, with the day's total and the fix.
pub fn within_day(spent: f64, worst: f64, ceiling: f64) -> Result<(), String> {
    if spent + worst <= ceiling {
        return Ok(());
    }
    Err(format!(
        "today's spend ${spent:.4} plus this run's worst case ${worst:.4} is over ux.daily_cost_usd ${ceiling:.2}; fix: wait until tomorrow (UTC), or raise the ceiling"
    ))
}

/// One run's line.
pub struct Run<'a> {
    pub time: &'a str,
    pub sha: &'a str,
    pub model: &'a str,
    pub usage: Usage,
    pub cost: f64,
}

/// Append `run` to the ledger, making the directory when it is missing.
///
/// # Errors
/// The ledger cannot be written, naming it.
pub fn record(ledger: &Path, run: &Run) -> Result<(), String> {
    let line = json!({ "time": run.time, "sha": run.sha, "model": run.model,
        "input": run.usage.input, "cached": run.usage.cached, "output": run.usage.output, "cost": run.cost });
    append(ledger, &line)
}

fn append(ledger: &Path, line: &Value) -> Result<(), String> {
    if let Some(dir) = ledger.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(ledger)
        .map_err(|e| format!("{}: {e}", ledger.display()))?;
    writeln!(file, "{line}").map_err(|e| format!("{}: {e}", ledger.display()))
}

#[cfg(test)]
#[path = "ux_ledger_tests.rs"]
mod tests;
