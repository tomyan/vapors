//! The order of a measurement, under the ledger's lock: count without a
//! frame, count each frame size at low and high detail (each count
//! charged at the input price), then book the smallest count's billed
//! check as `sent` under [`CAP_USD`] and the day's ceiling. The lock goes
//! before the billed request; its usage settles the booking. The raw
//! counts are kept.

use serde_json::json;

use crate::ux_answer::usage;
use crate::ux_config::UxConfig;
use crate::ux_count::count_url;
use crate::ux_estimate::dollars;
use crate::ux_ledger;
use crate::ux_ledger_write::{self as write, Entry, LOCK_WAIT, hold};
use crate::ux_measure::{CAP_USD, Count, body, count, distinct, now, row};
use crate::ux_run::Frame;
use crate::ux_secret::Secret;

/// Measure `ux.model`; returns the lines to print, the row last.
///
/// # Errors
/// A count or the billed check refused, or the billed count differing
/// from the endpoint's.
pub fn run(ux: &UxConfig, secret: &Secret) -> Result<Vec<String>, String> {
    let viewports = crate::ux_run::read(&ux.run_dir)?;
    let frames: Vec<&Frame> = viewports.iter().flat_map(|v| &v.frames).collect();
    let held = hold(&ux.report_dir, LOCK_WAIT)?;
    let spent = ux_ledger::spent_on(&ux.report_dir.join(ux_ledger::FILE), &now()[..10])?;
    let (counts, charged) = count_all(ux, secret, &frames)?;
    let smallest = counts
        .iter()
        .min_by_key(|c| c.with)
        .ok_or("the run has no frames")?;
    let frame = frames
        .iter()
        .find(|f| f.size == smallest.size)
        .ok_or("a size with no frame")?;
    let billed = check_billing(ux, secret, frame, smallest, spent + charged, held)?;
    let when = now();
    keep(ux, &when, &counts, billed)?;
    let name = ux.model.name;
    let mut lines: Vec<String> = counts
        .iter()
        .map(|c| {
            format!(
                "{name} {}x{} {}: {} with the frame, {} without: {} tokens, bound {}",
                c.size.0,
                c.size.1,
                c.detail,
                c.with,
                c.without,
                c.image(),
                c.bound()
            )
        })
        .collect();
    lines.push(format!(
        "one billed request used {billed} input tokens, as counted"
    ));
    lines.push(format!(
        "add to ux_models.rs: {}",
        row(name, &counts, &when)
    ));
    Ok(lines)
}

/// Every count, each charged in the ledger at the input price; returns
/// the counts and what they were charged.
fn count_all(
    ux: &UxConfig,
    secret: &Secret,
    frames: &[&Frame],
) -> Result<(Vec<Count>, f64), String> {
    let work = work_dir(ux)?;
    let (url, name) = (count_url(ux), ux.model.name);
    let ledger = ux.report_dir.join(ux_ledger::FILE);
    let mut charged = 0.0;
    let mut counted = |request: &serde_json::Value| -> Result<u64, String> {
        let n = count(&url, secret, &work, request)?;
        let time = now();
        let cost = dollars(n, ux.prices.input);
        write::count(
            &ledger,
            &Entry {
                time: &time,
                sha: "measure-image-cost",
                model: name,
                id: "",
            },
            n,
            cost,
        )?;
        charged += cost;
        Ok(n)
    };
    let without = counted(&body(name, None)?)?;
    let mut counts = Vec::new();
    for size in distinct(frames.iter().map(|f| f.size)) {
        let frame = frames
            .iter()
            .find(|f| f.size == size)
            .ok_or("a size with no frame")?;
        for detail in ["low", "high"] {
            let with = counted(&body(name, Some((frame, detail)))?)?;
            counts.push(Count::new(size, detail, with, without)?);
        }
    }
    Ok((counts, charged))
}

fn work_dir(ux: &UxConfig) -> Result<std::path::PathBuf, String> {
    let work = ux.report_dir.join("work");
    std::fs::create_dir_all(&work).map_err(|e| format!("{}: {e}", work.display()))?;
    Ok(work)
}

/// Send the smallest count's request for real: booked as `sent` under
/// [`CAP_USD`] and the day's ceiling while the ledger is `held`, then
/// settled by its usage, which must equal the count.
fn check_billing(
    ux: &UxConfig,
    secret: &Secret,
    frame: &Frame,
    counted: &Count,
    spent: f64,
    held: crate::ux_ledger_write::Held,
) -> Result<u64, String> {
    let worst = dollars(counted.with, ux.prices.input) + dollars(16, ux.prices.output);
    if worst > CAP_USD {
        return Err(format!(
            "the billed check could cost ${worst:.4}, over the measurement's ${CAP_USD:.2}; fix: measure a smaller frame"
        ));
    }
    ux_ledger::within_day(spent, worst, ux.caps.daily_usd)?;
    let ledger = ux.report_dir.join(ux_ledger::FILE);
    let time = now();
    let id = format!("{time}-{}", std::process::id());
    let entry = Entry {
        time: &time,
        sha: "measure-image-cost",
        model: ux.model.name,
        id: &id,
    };
    write::sent(&ledger, &entry, worst)?;
    drop(held);
    let response = billed_request(ux, secret, frame, counted.detail)?;
    let used = usage(&response)?;
    write::settled(&ledger, &entry, used, used.cost(&ux.prices))?;
    if used.input != counted.with {
        return Err(format!(
            "the count endpoint said {} input tokens, but the same request was billed {}; the counts are not what is billed",
            counted.with, used.input
        ));
    }
    Ok(used.input)
}

fn billed_request(
    ux: &UxConfig,
    secret: &Secret,
    frame: &Frame,
    detail: &str,
) -> Result<serde_json::Value, String> {
    let work = work_dir(ux)?;
    let mut request = body(ux.model.name, Some((frame, detail)))?;
    request["max_output_tokens"] = json!(16);
    request["store"] = json!(false);
    let file = work.join("billed.json");
    std::fs::write(&file, request.to_string()).map_err(|e| format!("{}: {e}", file.display()))?;
    crate::ux_call::post(
        &ux.endpoint,
        secret,
        &file,
        &work.join("billed-response.json"),
        crate::ux_call::DEADLINE,
    )
}

/// The raw counts, kept beside the reports.
fn keep(ux: &UxConfig, when: &str, counts: &[Count], billed: u64) -> Result<(), String> {
    let dir = ux.report_dir.join("image-cost");
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let raw: Vec<_> = counts.iter().map(|c| json!({ "size": [c.size.0, c.size.1], "detail": c.detail, "with": c.with, "without": c.without })).collect();
    let record =
        json!({ "model": ux.model.name, "time": when, "counts": raw, "billedCheck": billed });
    let path = dir.join(format!("{}-{}.json", ux.model.name, &when[..10]));
    std::fs::write(&path, record.to_string()).map_err(|e| format!("{}: {e}", path.display()))
}
