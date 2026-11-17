//! What an API response counted, read as it passes through the proxy
//! (`api_upstream.rs`). A streamed response carries usage in its events:
//! `message_start`'s message holds the input and cache counts and the
//! model billed, each `message_delta` the counts so far, and
//! `message_stop` says the stream ended whole. A response that is not a
//! stream holds them in its body's `usage` and `model`. A response is
//! counted only when it ended whole and every usage field it reports is
//! one the prices express (review of #476-#478, H2, M2, M3); otherwise
//! the proxy charges the request's worst case (`api_budget.rs`).

use serde_json::{Map, Value};

use crate::api_prices::Usage;

/// The counts the prices express.
const PRICED: [&str; 4] = [
    "input_tokens",
    "output_tokens",
    "cache_creation_input_tokens",
    "cache_read_input_tokens",
];

/// Usage fields that carry no price of their own: their values are
/// checked (`unpriced`), the rest are refused.
const DESCRIPTIVE: [&str; 6] = [
    "cache_creation",
    "server_tool_use",
    "service_tier",
    "inference_geo",
    "speed",
    "output_tokens_details",
];

/// What one response counted.
#[derive(Debug, PartialEq, Eq)]
pub enum Counted {
    /// It ended whole, at these counts, for this model.
    Whole(Usage, String),
    /// It did not end whole (cut, or not a body the proxy reads): why.
    /// Charged at the request's worst case, which bounds it.
    Cut(String),
    /// A body that is whole and is an API error, `{"type": "error"}`,
    /// with no usage: its error type. The API bills nothing for an error
    /// it answered (review of #476-#478, N2b), so the proxy charges 0
    /// when the status was not 2xx and curl ended well
    /// (`api_server_proxy.rs`); anything else is charged as cut.
    Error(String),
    /// It reported usage the prices cannot express: why. Charged at the
    /// worst case, and the session stops, as the worst case may not bound
    /// it.
    Unpriced(String),
}

/// The response read so far.
#[derive(Default)]
pub struct Scan {
    line: Vec<u8>,
    body: Vec<u8>,
    streamed: bool,
    stopped: bool,
    usage: Usage,
    model: Option<String>,
    unpriced: Option<String>,
}

impl Scan {
    /// Reads `bytes`, the next of the response's body.
    pub fn read(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            if byte == b'\n' {
                let line = std::mem::take(&mut self.line);
                self.event_line(&line);
            } else if self.line.len() < crate::api_http::BODY_LIMIT {
                self.line.push(byte);
            }
        }
        if !self.streamed && self.body.len() < crate::api_http::BODY_LIMIT {
            self.body.extend_from_slice(bytes);
        }
    }

    /// What the whole response counted.
    pub fn counted(&self) -> Counted {
        if let Some(why) = &self.unpriced {
            return Counted::Unpriced(why.clone());
        }
        if self.streamed {
            return match (&self.model, self.stopped) {
                (Some(model), true) => Counted::Whole(self.usage, model.clone()),
                _ => Counted::Cut("a stream that ended before message_stop".into()),
            };
        }
        whole_body(&self.body)
    }

    fn event_line(&mut self, line: &[u8]) {
        let Some(event) = line
            .strip_prefix(b"data: ")
            .and_then(|data| serde_json::from_slice::<Value>(data).ok())
        else {
            return;
        };
        match event["type"].as_str() {
            Some("message_start") if self.streamed => {
                self.unpriced = Some("a second message_start".into());
            }
            Some("message_start") => {
                self.streamed = true;
                self.model = event["message"]["model"].as_str().map(str::to_owned);
                self.add(&event["message"]["usage"]);
            }
            Some("message_delta") => self.add(&event["usage"]),
            Some("message_stop") => self.stopped = true,
            _ => {}
        }
    }

    /// The counts in `usage` taken (each reported is the total so far),
    /// or the response marked unknown if it holds an unpriced field.
    fn add(&mut self, usage: &Value) {
        let Some(map) = usage.as_object() else { return };
        if let Err(why) = unpriced(map) {
            self.unpriced = Some(why);
            return;
        }
        let count = |name: &str, was: u64| map.get(name).and_then(Value::as_u64).unwrap_or(was);
        let u = &mut self.usage;
        (u.input, u.output) = (count(PRICED[0], u.input), count(PRICED[1], u.output));
        (u.cache_write, u.cache_read) = (
            count(PRICED[2], u.cache_write),
            count(PRICED[3], u.cache_read),
        );
    }
}

fn whole_body(body: &[u8]) -> Counted {
    let Ok(value) = serde_json::from_slice::<Value>(body) else {
        return Counted::Cut("a response body that is not JSON".into());
    };
    if value["type"] == "error" && value.get("usage").is_none() {
        let kind = value["error"]["type"].as_str().unwrap_or_default();
        return Counted::Error(crate::api_refusals::escaped(kind));
    }
    let (Some(usage), Some(model)) = (value["usage"].as_object(), value["model"].as_str()) else {
        return Counted::Cut("a response with no usage or no model".into());
    };
    if let Err(why) = unpriced(usage) {
        return Counted::Unpriced(why);
    }
    let count = |name: &str| usage.get(name).and_then(Value::as_u64).unwrap_or(0);
    let counted = Usage {
        input: count(PRICED[0]),
        output: count(PRICED[1]),
        cache_write: count(PRICED[2]),
        cache_read: count(PRICED[3]),
    };
    Counted::Whole(counted, model.to_owned())
}

/// Refuses a usage field the prices cannot express: an unknown field,
/// an hour-long cache write, a server tool's use, a tier, region or
/// speed with a price of its own, or `iterations` that is not empty:
/// sampling a compaction ran, billed apart from the four counts, which
/// the proxy does not sum (review of #476-#478, N2b).
fn unpriced(usage: &Map<String, Value>) -> Result<(), String> {
    for (field, value) in usage {
        let fine = PRICED.contains(&field.as_str())
            || match field.as_str() {
                "cache_creation" => zero(&value["ephemeral_1h_input_tokens"]),
                "server_tool_use" => value.as_object().is_none_or(|uses| uses.values().all(zero)),
                "service_tier" | "speed" => value.is_null() || value == "standard",
                "inference_geo" => value.is_null() || value == "global" || value == "not_available",
                "iterations" => value.is_null() || value.as_array().is_some_and(Vec::is_empty),
                name => DESCRIPTIVE.contains(&name),
            };
        if !fine {
            return Err(format!(
                "the usage field {field:?} = {} is not one the proxy prices",
                crate::api_refusals::escaped(&value.to_string())
            ));
        }
    }
    Ok(())
}

fn zero(value: &Value) -> bool {
    value.is_null() || value.as_u64() == Some(0)
}

#[cfg(test)]
#[path = "api_usage_tests.rs"]
mod tests;
