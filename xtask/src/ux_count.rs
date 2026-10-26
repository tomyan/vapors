//! The exact input count of a review's request, from the Responses
//! API's `input_tokens` endpoint
//! (`developers.openai.com/api/reference/resources/responses/subresources/input_tokens`,
//! read 2026-10-26),
//! taken before the request is sent. The byte bound (`ux_estimate`) is
//! the first check; the count turns the caps from an estimate into a
//! measurement. A count that fails refuses the run: it never falls back
//! to the estimate.

use std::path::Path;

use crate::ux_config::UxConfig;
use crate::ux_estimate::dollars;
use crate::ux_secret::Secret;

/// The counting endpoint beside `ux.endpoint` (`/responses` →
/// `/responses/input_tokens`).
pub fn count_url(ux: &UxConfig) -> String {
    format!("{}/input_tokens", ux.endpoint.trim_end_matches('/'))
}

/// Fields of a review's request that only shape its output, which the
/// count endpoint refuses ("Unknown parameter: 'store'", HTTP 400,
/// 2026-10-26). Taking them out leaves the input the same.
const OUTPUT_ONLY: [&str; 2] = ["store", "max_output_tokens"];

/// The input tokens of the request in `request`, counted by the API.
///
/// # Errors
/// The request unreadable, the call's refusal, or an answer with no
/// `input_tokens`.
pub fn exact(ux: &UxConfig, secret: &Secret, request: &Path) -> Result<u64, String> {
    let text =
        std::fs::read_to_string(request).map_err(|e| format!("{}: {e}", request.display()))?;
    let mut body: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", request.display()))?;
    let fields = body
        .as_object_mut()
        .ok_or("the request is not a JSON object")?;
    for field in OUTPUT_ONLY {
        fields.remove(field);
    }
    let count_file = request.with_file_name("count-request.json");
    std::fs::write(&count_file, body.to_string())
        .map_err(|e| format!("{}: {e}", count_file.display()))?;
    let answer_file = request.with_file_name("count-response.json");
    let answer = crate::ux_call::post(
        &count_url(ux),
        secret,
        &count_file,
        &answer_file,
        crate::ux_call::DEADLINE,
    )?;
    input_tokens(&answer)
}

/// The count in an `input_tokens` answer, refused unless it is one, of
/// at least one token: a count of 0 would pin nothing.
///
/// # Errors
/// Another object, or no positive `input_tokens`.
pub fn input_tokens(answer: &serde_json::Value) -> Result<u64, String> {
    if answer["object"] != "response.input_tokens" {
        return Err(format!(
            "the count's answer is not a `response.input_tokens`: {answer}"
        ));
    }
    answer["input_tokens"]
        .as_u64()
        .filter(|n| *n > 0)
        .ok_or_else(|| {
            format!("the count's `input_tokens` is not a positive whole number: {answer}")
        })
}

/// The caps checked against the exact count.
///
/// # Errors
/// The count over the estimate that should bound it, over the input
/// cap, or making the worst case over the run's cap or the day's.
pub fn within(ux: &UxConfig, estimate: u64, counted: u64, spent_today: f64) -> Result<f64, String> {
    if counted > estimate {
        return Err(format!(
            "the API counted {counted} input tokens, over the estimate of {estimate} that should bound it; fix: correct ux_estimate.rs or the model's row in ux_models.rs"
        ));
    }
    if counted > ux.caps.input_tokens {
        return Err(format!(
            "the API counted {counted} input tokens, over ux.max_input_tokens {}; fix: send fewer frames or design files",
            ux.caps.input_tokens
        ));
    }
    let worst =
        dollars(counted, ux.prices.input) + dollars(ux.caps.output_tokens, ux.prices.output);
    if worst > ux.caps.cost_usd {
        return Err(format!(
            "at its counted {counted} input tokens the request could cost ${worst:.4}, over ux.max_cost_usd ${:.2}; fix: lower ux.max_output_tokens or send less",
            ux.caps.cost_usd
        ));
    }
    crate::ux_ledger::within_day(spent_today, worst, ux.caps.daily_usd)?;
    Ok(worst)
}

#[cfg(test)]
#[path = "ux_count_tests.rs"]
mod tests;
