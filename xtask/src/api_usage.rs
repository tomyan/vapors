//! The tokens an API response counted, read as it passes through the
//! proxy (`api_upstream.rs`). A streamed response carries them in its
//! events: `message_start`'s message holds the input and cache counts,
//! and each `message_delta` the output so far. A response that is not a
//! stream holds them in its body's `usage`.

use serde_json::Value;

use crate::api_prices::Usage;

/// The usage seen so far in one response.
#[derive(Default)]
pub struct Scan {
    line: Vec<u8>,
    body: Vec<u8>,
    streamed: bool,
    usage: Usage,
}

impl Scan {
    /// Reads `bytes`, the next of the response's body.
    pub fn read(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            if byte == b'\n' {
                let line = std::mem::take(&mut self.line);
                self.event_line(&line);
            } else {
                self.line.push(byte);
            }
        }
        if !self.streamed && self.body.len() < crate::api_http::BODY_LIMIT {
            self.body.extend_from_slice(bytes);
        }
    }

    /// The response's usage, its body read to the end; `None` when it
    /// holds none (no `message_start`, or a body with no `usage`), which
    /// for a successful response leaves the budget uncountable.
    pub fn usage(&self) -> Option<Usage> {
        if self.streamed {
            return Some(self.usage);
        }
        let body: Value = serde_json::from_slice(&self.body).ok()?;
        let counts = body.get("usage").filter(|usage| usage.is_object())?;
        let mut usage = Usage::default();
        add_counts(&mut usage, counts);
        Some(usage)
    }

    fn event_line(&mut self, line: &[u8]) {
        let Some(data) = line.strip_prefix(b"data: ") else {
            return;
        };
        let Ok(event) = serde_json::from_slice::<Value>(data) else {
            return;
        };
        match event["type"].as_str() {
            Some("message_start") => {
                self.streamed = true;
                add_counts(&mut self.usage, &event["message"]["usage"]);
            }
            Some("message_delta") => {
                if let Some(output) = event["usage"]["output_tokens"].as_u64() {
                    self.usage.output = output;
                }
            }
            _ => {}
        }
    }
}

/// `usage`'s counts set from an API `usage` object.
fn add_counts(usage: &mut Usage, from: &Value) {
    let count = |name: &str| from[name].as_u64().unwrap_or(0);
    usage.input = count("input_tokens");
    usage.output = count("output_tokens");
    usage.cache_write = count("cache_creation_input_tokens");
    usage.cache_read = count("cache_read_input_tokens");
}

#[cfg(test)]
#[path = "api_usage_tests.rs"]
mod tests;
