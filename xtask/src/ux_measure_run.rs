//! The order of a measurement. The count bodies are built first and their
//! summed bound checked against the day's ceiling (review-v69b N2); then
//! each count is booked before it is sent and closed by what it counted
//! (`ux_spend::count`); then the smallest count's billed check is booked
//! under [`CAP_USD`] and settled by its usage, which must equal the
//! count. The raw counts are kept.

use serde_json::{Value, json};

use crate::ux_answer::usage;
use crate::ux_config::UxConfig;
use crate::ux_count::count_url;
use crate::ux_estimate::{dollars, text_tokens};
use crate::ux_measure::{CAP_USD, Count, body, count, distinct, now, row};
use crate::ux_run::Frame;
use crate::ux_secret::Secret;
use crate::ux_spend;

/// The ledger's `sha` for a measurement's lines.
const SHA: &str = "measure-image-cost";

/// Measure `ux.model`; returns the lines to print, the row last.
///
/// # Errors
/// A booking, a count or the billed check refused, or the billed count
/// differing from the endpoint's.
pub fn run(ux: &UxConfig, secret: &Secret) -> Result<Vec<String>, String> {
    let viewports = crate::ux_run::read(&ux.run_dir)?;
    let frames: Vec<&Frame> = viewports.iter().flat_map(|v| &v.frames).collect();
    let asks = asks(ux.model.name, &frames)?;
    let bound: u64 = asks
        .iter()
        .map(|(_, body)| text_tokens(body.to_string().len()))
        .sum();
    ux_spend::room(ux, dollars(bound, ux.prices.input))?;
    let counts = count_all(ux, secret, &asks)?;
    let smallest = counts
        .iter()
        .min_by_key(|c| c.with)
        .ok_or("the run has no frames")?;
    let frame = frames
        .iter()
        .find(|f| f.size == smallest.size)
        .ok_or("a size with no frame")?;
    let billed = check_billing(ux, secret, frame, smallest)?;
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

/// What each count asks: none for the body without a frame, then each
/// frame size at low and high detail.
type Ask<'a> = (Option<(&'a Frame, &'static str)>, Value);

fn asks<'a>(model: &str, frames: &[&'a Frame]) -> Result<Vec<Ask<'a>>, String> {
    let mut asks = vec![(None, body(model, None)?)];
    for size in distinct(frames.iter().map(|f| f.size)) {
        let frame = *frames
            .iter()
            .find(|f| f.size == size)
            .ok_or("a size with no frame")?;
        for detail in ["low", "high"] {
            asks.push((Some((frame, detail)), body(model, Some((frame, detail)))?));
        }
    }
    Ok(asks)
}

/// Every count, each booked at its body's byte bound before it is sent.
fn count_all(ux: &UxConfig, secret: &Secret, asks: &[Ask]) -> Result<Vec<Count>, String> {
    let (work, url) = (work_dir(ux)?, count_url(ux));
    let mut found = Vec::new();
    for (frame, request) in asks {
        let bound = text_tokens(request.to_string().len());
        found.push((
            frame,
            ux_spend::count(ux, SHA, bound, 0.0, || count(&url, secret, &work, request))?,
        ));
    }
    let without = found.first().map(|(_, n)| *n).ok_or("no count was made")?;
    found
        .iter()
        .filter_map(|(frame, n)| frame.map(|(f, detail)| Count::new(f.size, detail, *n, without)))
        .collect()
}

fn work_dir(ux: &UxConfig) -> Result<std::path::PathBuf, String> {
    let work = ux.report_dir.join("work");
    std::fs::create_dir_all(&work).map_err(|e| format!("{}: {e}", work.display()))?;
    Ok(work)
}

/// Send the smallest count's request for real: booked under [`CAP_USD`]
/// and the day's ceiling, settled by its usage, which must equal the
/// count.
fn check_billing(
    ux: &UxConfig,
    secret: &Secret,
    frame: &Frame,
    counted: &Count,
) -> Result<u64, String> {
    let worst = dollars(counted.with, ux.prices.input) + dollars(16, ux.prices.output);
    if worst > CAP_USD {
        return Err(format!(
            "the billed check could cost ${worst:.4}, over the measurement's ${CAP_USD:.2}; fix: measure a smaller frame"
        ));
    }
    let booked = ux_spend::book(ux, SHA, worst, 0.0)?;
    let used = usage(&billed_request(ux, secret, frame, counted.detail)?)?;
    ux_spend::settle(ux, &booked, used, counted.with)?;
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
) -> Result<Value, String> {
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
