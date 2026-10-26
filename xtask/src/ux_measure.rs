//! `cargo xtask ux-review --measure-image-cost`: how many input tokens
//! `ux.model` counts for each frame size the walk records, at low and
//! high detail, for a model `OpenAI` documents no formula for. Each count
//! is the Responses API's `input_tokens` endpoint with the frame minus
//! without it; it generates nothing. One real request, the smallest,
//! checks that the endpoint's count is what is billed. It prints the row
//! for `ux_models`, each bound the count plus 20%, and keeps the raw
//! counts in `<report_dir>/image-cost/`.

use std::path::Path;
use std::time::SystemTime;

use serde_json::{Value, json};

use crate::ux_run::Frame;
use crate::ux_secret::Secret;

/// The most the measurement may spend on its one real request
/// (the lead, 2026-10-26).
pub const CAP_USD: f64 = 0.05;

/// What the counting requests ask.
const TEXT: &str = "Name the main colour in this image in one word.";

/// One frame size's counts, with the frame and without.
#[derive(Debug, PartialEq, Eq)]
pub struct Count {
    pub size: (u32, u32),
    pub detail: &'static str,
    pub with: u64,
    pub without: u64,
}

impl Count {
    pub fn image(&self) -> u64 {
        self.with - self.without
    }

    /// The row's bound: the count plus 20%, rounded up.
    pub fn bound(&self) -> u64 {
        (self.image() * 12).div_ceil(10)
    }
}

/// The request body: the text, and `frame` at `detail` when given.
///
/// # Errors
/// A frame that cannot be read.
pub fn body(model: &str, frame: Option<(&Frame, &str)>) -> Result<Value, String> {
    let mut content = vec![json!({ "type": "input_text", "text": TEXT })];
    if let Some((frame, detail)) = frame {
        let png =
            std::fs::read(&frame.path).map_err(|e| format!("{}: {e}", frame.path.display()))?;
        let url = format!("data:image/png;base64,{}", crate::ux_base64::encode(&png));
        content.push(json!({ "type": "input_image", "image_url": url, "detail": detail }));
    }
    Ok(json!({ "model": model, "input": [{ "role": "user", "content": content }] }))
}

/// Each size once, in the order first seen.
pub fn distinct(all: impl Iterator<Item = (u32, u32)>) -> Vec<(u32, u32)> {
    let mut found: Vec<(u32, u32)> = Vec::new();
    for size in all {
        if !found.contains(&size) {
            found.push(size);
        }
    }
    found
}

/// Count `body`'s input tokens at the endpoint `count_url`.
///
/// # Errors
/// The call's refusal, or an answer with no `input_tokens`.
pub fn count(count_url: &str, secret: &Secret, work: &Path, body: &Value) -> Result<u64, String> {
    let file = work.join("count.json");
    std::fs::write(&file, body.to_string()).map_err(|e| format!("{}: {e}", file.display()))?;
    let answer = crate::ux_call::post(
        count_url,
        secret,
        &file,
        &work.join("count-response.json"),
        crate::ux_call::DEADLINE,
    )?;
    answer["input_tokens"]
        .as_u64()
        .ok_or_else(|| format!("the count has no `input_tokens`: {answer}"))
}

/// The row to add to `ux_models`, and its source with the raw counts.
pub fn row(model: &str, counts: &[Count], when: &str) -> String {
    let sizes: Vec<String> = distinct(counts.iter().map(|c| c.size))
        .iter()
        .map(|&(w, h)| {
            let at = |detail: &str| {
                counts
                    .iter()
                    .find(|c| c.size == (w, h) && c.detail == detail)
                    .map_or(0, Count::bound)
            };
            format!(
                "Measured {{ size: ({w}, {h}), low: {}, high: {} }}",
                at("low"),
                at("high")
            )
        })
        .collect();
    let raw: Vec<String> = counts
        .iter()
        .map(|c| {
            format!(
                "{}x{} {} {}-{}",
                c.size.0, c.size.1, c.detail, c.with, c.without
            )
        })
        .collect();
    format!(
        "Model {{ name: \"{model}\", images: Images::Measured {{ sizes: &[{}], source: \"--measure-image-cost {when}: {}\" }} }},",
        sizes.join(", "),
        raw.join("; ")
    )
}

/// The time the measurement ran, for its source.
pub fn now() -> String {
    crate::ux_ledger::utc(SystemTime::now())
}

#[cfg(test)]
#[path = "ux_measure_tests.rs"]
mod tests;
