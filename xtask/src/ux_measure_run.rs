//! The order of a measurement: count without a frame, count each frame
//! size at low and high detail, check the smallest count against one
//! billed request held under [`CAP_USD`], record its spend, and keep the
//! raw counts.

use std::path::Path;

use serde_json::json;

use crate::ux_answer::usage;
use crate::ux_config::UxConfig;
use crate::ux_count::count_url;
use crate::ux_estimate::dollars;
use crate::ux_ledger::{self, Run};
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
    let work = ux.report_dir.join("work");
    std::fs::create_dir_all(&work).map_err(|e| format!("{}: {e}", work.display()))?;
    let (url, name) = (count_url(ux), ux.model.name);
    let ledger = ux.report_dir.join(ux_ledger::FILE);
    let counted = |request: &serde_json::Value| -> Result<u64, String> {
        let n = count(&url, secret, &work, request)?;
        ux_ledger::record_count(&ledger, &now(), "measure-image-cost", name, n)?;
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
            counts.push(Count {
                size,
                detail,
                with,
                without,
            });
        }
    }
    let smallest = counts
        .iter()
        .min_by_key(|c| c.with)
        .ok_or("the run has no frames")?;
    let frame = frames
        .iter()
        .find(|f| f.size == smallest.size)
        .ok_or("a size with no frame")?;
    let billed = check_billing(ux, secret, &work, frame, smallest)?;
    let when = now();
    keep(ux, &when, &counts, billed)?;
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

/// Send the smallest count's request for real, held under [`CAP_USD`]
/// and the day's ceiling; its billed input must equal the count.
fn check_billing(
    ux: &UxConfig,
    secret: &Secret,
    work: &Path,
    frame: &Frame,
    counted: &Count,
) -> Result<u64, String> {
    let worst = dollars(counted.with, ux.prices.input) + dollars(16, ux.prices.output);
    if worst > CAP_USD {
        return Err(format!(
            "the billed check could cost ${worst:.4}, over the measurement's ${CAP_USD:.2}; fix: measure a smaller frame"
        ));
    }
    let time = now();
    let ledger = ux.report_dir.join(ux_ledger::FILE);
    ux_ledger::within_day(
        ux_ledger::spent_on(&ledger, &time[..10])?,
        worst,
        ux.caps.daily_usd,
    )?;
    let mut request = body(ux.model.name, Some((frame, counted.detail)))?;
    request["max_output_tokens"] = json!(16);
    request["store"] = json!(false);
    let file = work.join("billed.json");
    std::fs::write(&file, request.to_string()).map_err(|e| format!("{}: {e}", file.display()))?;
    let response = crate::ux_call::post(
        &ux.endpoint,
        secret,
        &file,
        &work.join("billed-response.json"),
        crate::ux_call::DEADLINE,
    )?;
    let used = usage(&response)?;
    let run = Run {
        time: &time,
        sha: "measure-image-cost",
        model: ux.model.name,
        usage: used,
        cost: used.cost(&ux.prices),
    };
    ux_ledger::record(&ledger, &run)?;
    if used.input != counted.with {
        return Err(format!(
            "the count endpoint said {} input tokens, but the same request was billed {}; the counts are not what is billed",
            counted.with, used.input
        ));
    }
    Ok(used.input)
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
