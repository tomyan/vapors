//! `spend.jsonl` in the report directory, and the day's spend read from
//! it. Every line has a time (UTC), a model, a `kind` and a cost:
//! - `count`: an `input_tokens` request, at the input price, since the
//!   documentation does not say counting is free;
//! - `sent`: a request about to be sent, at its worst case, written
//!   before curl starts, with an `id`;
//! - `settled`: what that request cost, by its response's usage, with the
//!   same `id`.
//!
//! A `sent` line with no `settled` line counts at its worst case: a
//! timeout, a dropped connection or a response with no usage may still
//! have been billed. Before a review is sent, the day's spend plus its
//! worst case must stay under `ux.daily_cost_usd`. Writing is
//! `ux_ledger_write`.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

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

/// The cost of every line recorded on `date` (`YYYY-MM-DD`), each
/// `sent` line at its worst case unless a `settled` line has its `id`;
/// none when the ledger does not exist yet.
///
/// # Errors
/// An unreadable ledger, or a line that is not one of the kinds, naming
/// the line; a torn last line, naming the fix.
pub fn spent_on(ledger: &Path, date: &str) -> Result<f64, String> {
    let text = match std::fs::read_to_string(ledger) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(0.0),
        Err(e) => return Err(format!("{}: {e}", ledger.display())),
    };
    let lines = parse(ledger, &text)?;
    let settled: Vec<&str> = lines
        .iter()
        .filter(|l| l.kind == "settled")
        .filter_map(|l| l.id.as_deref())
        .collect();
    let open =
        |l: &&Line| l.kind != "sent" || !l.id.as_deref().is_some_and(|id| settled.contains(&id));
    Ok(lines
        .iter()
        .filter(|l| l.time.starts_with(date))
        .filter(open)
        .map(|l| l.cost)
        .sum())
}

/// One line of the ledger, as `spent_on` reads it.
struct Line {
    time: String,
    kind: String,
    id: Option<String>,
    cost: f64,
}

fn parse(ledger: &Path, text: &str) -> Result<Vec<Line>, String> {
    let count = text.lines().count();
    let torn = !text.is_empty() && !text.ends_with('\n');
    text.lines().enumerate().map(|(index, raw)| {
        let place = format!("{}:{}", ledger.display(), index + 1);
        line(raw).map_err(|why| {
            if torn && index + 1 == count {
                format!("{place}: the last line is torn ({why}), a write cut short; fix: delete that line once its request is accounted for")
            } else {
                format!("{place}: {why}")
            }
        })
    }).collect()
}

fn line(raw: &str) -> Result<Line, String> {
    let value: Value = serde_json::from_str(raw).map_err(|e| format!("not a ledger line: {e}"))?;
    let text = |key: &str| value[key].as_str().map(str::to_owned);
    let kind = text("kind").filter(|k| ["count", "sent", "settled"].contains(&k.as_str()));
    match (text("time"), kind, value["cost"].as_f64()) {
        (Some(time), Some(kind), Some(cost)) if kind == "count" || text("id").is_some() => Ok(Line { time, kind, id: text("id"), cost }),
        _ => Err("a line with no `time`, `cost`, or `kind` of count, sent or settled (and an `id` on the last two)".to_owned()),
    }
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

#[cfg(test)]
#[path = "ux_ledger_tests.rs"]
mod tests;
