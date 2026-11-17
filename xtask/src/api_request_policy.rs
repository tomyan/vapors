//! What an API request may ask for, judged before it is forwarded
//! (review of #476-#478, H1 and M2). The session writes the request, so
//! the proxy forwards only what it can count: one of the targets the CLI
//! sends, whole (curl is never handed a target it could expand into
//! more requests), and a body whose every priced choice the four prices
//! of `[api_build.prices]` express. Anything else is refused, naming it.

use serde_json::Value;

/// The targets forwarded, query included, as the CLI sends them
/// (recorded 2026-11-17, claude 2.1.323).
pub const TARGETS: [&str; 4] = [
    "/v1/messages",
    "/v1/messages?beta=true",
    "/v1/messages/count_tokens",
    "/v1/messages/count_tokens?beta=true",
];

/// The body's top-level fields forwarded: those the CLI sends (recorded
/// as above), none of which changes the price of a token. Absent here
/// and so refused: `speed`, `service_tier`, `inference_geo`,
/// `mcp_servers`, `container` and anything new.
const FIELDS: [&str; 13] = [
    "model",
    "max_tokens",
    "messages",
    "system",
    "tools",
    "tool_choice",
    "metadata",
    "stream",
    "thinking",
    "output_config",
    "context_management",
    "temperature",
    "stop_sequences",
];

/// What a message request asks for, for its worst case
/// (`api_prices.rs` `Price::worst`).
#[derive(Debug, PartialEq, Eq)]
pub struct Demand {
    pub model: String,
    /// The most output tokens it may be billed for.
    pub max_tokens: u64,
    /// An upper bound on its input tokens: the body's bytes, as a token
    /// is at least one byte of the text or data it stands for.
    pub input_bound: u64,
}

/// `target` and `body` judged; what a message asks for, or `None` for a
/// token count, which is not billed.
///
/// # Errors
///
/// A target not in [`TARGETS`], a body that is not a JSON object, a
/// field not in [`FIELDS`], a server tool, a cache write other than the
/// five-minute one, or no `model` or `max_tokens`, each named.
pub fn judged(target: &str, body: &[u8]) -> Result<Option<Demand>, String> {
    if !TARGETS.contains(&target) {
        return Err(format!(
            "the target {target:?} is not one the proxy forwards ({})",
            TARGETS.join(", ")
        ));
    }
    let value: Value = serde_json::from_slice(body)
        .map_err(|e| format!("a request body that is not JSON: {e}"))?;
    let object = value
        .as_object()
        .ok_or("a request body that is not a JSON object")?;
    if let Some(field) = object.keys().find(|key| !FIELDS.contains(&key.as_str())) {
        return Err(format!(
            "the request field {field:?} is not one the proxy can price"
        ));
    }
    server_tools(&value["tools"])?;
    cache_writes(&value)?;
    if target.starts_with("/v1/messages/count_tokens") {
        return Ok(None);
    }
    demand(&value, body.len()).map(Some)
}

/// Refuses a server tool: one with a `type`, billed per use, which the
/// prices do not express. The CLI's tools are custom: no `type`.
fn server_tools(tools: &Value) -> Result<(), String> {
    let server = tools
        .as_array()
        .into_iter()
        .flatten()
        .find(|tool| tool.get("type").is_some_and(|t| t != "custom"));
    match server {
        Some(tool) => Err(format!(
            "the server tool {} is billed per use, which the proxy cannot price",
            tool["type"]
        )),
        None => Ok(()),
    }
}

/// Refuses a cache write the prices do not express: any `cache_control`
/// but `{"type": "ephemeral"}` with no `ttl` or a `ttl` of `5m`.
fn cache_writes(value: &Value) -> Result<(), String> {
    match value {
        Value::Object(map) => {
            if let Some(control) = map.get("cache_control") {
                let ttl = control.get("ttl").and_then(Value::as_str).unwrap_or("5m");
                if control.get("type").and_then(Value::as_str) != Some("ephemeral") || ttl != "5m" {
                    return Err(format!(
                        "the cache_control {control} is not the five-minute cache write the proxy prices"
                    ));
                }
            }
            map.values().try_for_each(cache_writes)
        }
        Value::Array(items) => items.iter().try_for_each(cache_writes),
        _ => Ok(()),
    }
}

fn demand(value: &Value, bytes: usize) -> Result<Demand, String> {
    let model = value["model"]
        .as_str()
        .ok_or("a request with no `model` string")?
        .to_owned();
    let max_tokens = value["max_tokens"]
        .as_u64()
        .ok_or("a request with no whole `max_tokens`")?;
    let input_bound = u64::try_from(bytes).map_err(|_| "a body too long to count")?;
    Ok(Demand {
        model,
        max_tokens,
        input_bound,
    })
}

#[cfg(test)]
#[path = "api_request_policy_tests.rs"]
mod tests;
