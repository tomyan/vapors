//! The Responses API's answer: its usage, read first so the spend is
//! recorded whatever else is wrong, then the judgements, refused unless
//! each judged item is answered once, citing only frames that were sent.

use serde_json::Value;

use crate::ux_checklist::Kind;
use crate::ux_config::Prices;
use crate::ux_estimate::dollars;
use crate::ux_review::Prepared;

/// The tokens a response used (field names from the API reference,
/// developers.openai.com/api/reference/resources/responses, read
/// 2026-10-26).
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Usage {
    pub input: u64,
    /// Of `input`; absent from a response means none were cached.
    pub cached: u64,
    /// Including reasoning tokens.
    pub output: u64,
}

impl Usage {
    /// What the tokens cost at `prices`.
    pub fn cost(&self, prices: &Prices) -> f64 {
        dollars(self.input - self.cached, prices.input)
            + dollars(self.cached, prices.cached)
            + dollars(self.output, prices.output)
    }
}

/// One judged item's answer.
#[derive(Debug, PartialEq, Eq)]
pub struct Judgement {
    pub item: String,
    pub verdict: String,
    pub severity: u64,
    pub frames: Vec<String>,
    pub note: String,
}

/// The response's usage.
///
/// # Errors
/// No `usage`, or token counts that are not whole numbers.
pub fn usage(response: &Value) -> Result<Usage, String> {
    let count = |at: &Value, what: &str| {
        at.as_u64().ok_or_else(|| {
            format!(
                "the response's usage has no `{what}`: {}",
                response["usage"]
            )
        })
    };
    let usage = &response["usage"];
    let cached = match &usage["input_tokens_details"]["cached_tokens"] {
        Value::Null => 0,
        other => count(other, "input_tokens_details.cached_tokens")?,
    };
    let input = count(&usage["input_tokens"], "input_tokens")?;
    if cached > input {
        return Err(format!(
            "the response says {cached} of {input} input tokens were cached"
        ));
    }
    Ok(Usage {
        input,
        cached,
        output: count(&usage["output_tokens"], "output_tokens")?,
    })
}

/// The judgements in a completed response.
///
/// # Errors
/// A response that did not complete, a refusal, no single JSON answer,
/// or judgements that do not match what was asked.
pub fn judgements(response: &Value, prepared: &Prepared) -> Result<Vec<Judgement>, String> {
    let text = answer_text(response)?;
    let answer: Value =
        serde_json::from_str(&text).map_err(|e| format!("the answer is not JSON: {e}"))?;
    let list = answer["judgements"]
        .as_array()
        .ok_or("the answer has no `judgements` list")?;
    let judged: Vec<&str> = prepared
        .items
        .iter()
        .filter(|i| i.kind == Kind::Judged)
        .map(|i| i.id.as_str())
        .collect();
    let frames: Vec<&str> = prepared
        .viewports
        .iter()
        .flat_map(|v| &v.frames)
        .map(|f| f.name.as_str())
        .collect();
    let found = list
        .iter()
        .map(|j| judgement(j, &frames))
        .collect::<Result<Vec<_>, _>>()?;
    for id in &judged {
        let times = found.iter().filter(|j| j.item == *id).count();
        if times != 1 {
            return Err(format!("the answer judges `{id}` {times} times, not once"));
        }
    }
    if let Some(stray) = found.iter().find(|j| !judged.contains(&j.item.as_str())) {
        return Err(format!(
            "the answer judges `{}`, which is not a judged item",
            stray.item
        ));
    }
    Ok(found)
}

/// The one `output_text` of a completed response.
fn answer_text(response: &Value) -> Result<String, String> {
    if response["status"] != "completed" {
        return Err(format!(
            "the response is `{}`, not completed: {} {}",
            response["status"], response["incomplete_details"], response["error"]
        ));
    }
    let parts: Vec<&Value> = response["output"]
        .as_array()
        .map_or(&[][..], Vec::as_slice)
        .iter()
        .filter(|item| item["type"] == "message")
        .flat_map(|item| item["content"].as_array().map_or(&[][..], Vec::as_slice))
        .collect();
    if let Some(refusal) = parts.iter().find(|p| p["type"] == "refusal") {
        return Err(format!("the model refused: {}", refusal["refusal"]));
    }
    let texts: Vec<&str> = parts
        .iter()
        .filter(|p| p["type"] == "output_text")
        .filter_map(|p| p["text"].as_str())
        .collect();
    match texts.as_slice() {
        [text] => Ok((*text).to_owned()),
        other => Err(format!(
            "the response has {} output texts, not one",
            other.len()
        )),
    }
}

fn judgement(value: &Value, frames: &[&str]) -> Result<Judgement, String> {
    let text = |key: &str| {
        value[key]
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| format!("a judgement has no `{key}`: {value}"))
    };
    let verdict = text("verdict")?;
    if !crate::ux_request::VERDICTS.contains(&verdict.as_str()) {
        return Err(format!(
            "a judgement's verdict `{verdict}` is not one of {:?}",
            crate::ux_request::VERDICTS
        ));
    }
    let severity = value["severity"]
        .as_u64()
        .filter(|s| (1..=3).contains(s))
        .ok_or_else(|| format!("a judgement's severity is not 1, 2 or 3: {value}"))?;
    let cited: Vec<String> = value["frames"]
        .as_array()
        .ok_or_else(|| format!("a judgement has no `frames`: {value}"))?
        .iter()
        .map(|f| {
            f.as_str()
                .filter(|f| frames.contains(f))
                .map(str::to_owned)
                .ok_or_else(|| format!("a judgement cites {f}, which was not sent"))
        })
        .collect::<Result<_, _>>()?;
    if cited.is_empty() && verdict != "cannot-judge" {
        return Err(format!(
            "a judgement with the verdict `{verdict}` cites no frame: {value}"
        ));
    }
    let note = text("note")?;
    crate::ux_report::check_note(&note)?;
    Ok(Judgement {
        item: text("item")?,
        verdict,
        severity,
        frames: cited,
        note,
    })
}

#[cfg(test)]
#[path = "ux_answer_tests.rs"]
mod tests;
