//! What an API request may ask for, judged before it is forwarded
//! (review of #476-#478, H1 and M2). The session writes the request, so
//! the proxy forwards only what it can count: one of the targets the CLI
//! sends, whole (curl is never handed a target it could expand into
//! more requests), and a body whose every priced choice the four prices
//! of `[api_build.prices]` express. Anything else is refused, naming it.
//! The body is parsed once (`api_json_strict.rs`) and that parse is what
//! is forwarded, so the request judged is the request the API reads.

use serde_json::Value;

use crate::api_json_strict::parsed;
use crate::api_request_content::plain;

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

/// The `anthropic-beta` values forwarded: those the CLI sends (recorded
/// 2026-11-17, claude 2.1.323, `--bare`). A beta can change what a token
/// costs with no body field to refuse (review of #476-#478, N2b), so any
/// other is refused. `context-management-2025-06-27` is among them: the
/// CLI's `context_management` clears earlier tool results, and a
/// response's `usage.iterations`, which would carry billed sampling not
/// in the four counts, is refused unless empty (`api_usage.rs`).
pub const BETAS: [&str; 7] = [
    "claude-code-20250219",
    "interleaved-thinking-2025-05-14",
    "context-management-2025-06-27",
    "per-turn-control-2026-07-01",
    "advisor-tool-2026-03-01",
    "thinking-token-count-2026-05-13",
    "effort-2025-11-24",
];

/// The most tokens of system prompt the API adds when `tools` is
/// present: the largest figure in the "Tool use system prompt tokens"
/// table (Claude Opus 4.7, `any`/`tool`), in the Pricing section of
/// <https://platform.claude.com/docs/en/agents-and-tools/tool-use/overview>
/// (read 2026-11-17; Sonnet 5.5 and Opus 5.5 list 286). Not in the
/// body's bytes, so added to the input bound (review of #476-#478, N1).
pub const TOOL_PROMPT_TOKENS: u64 = 804;

/// Refuses an `anthropic-beta` header naming a value not in [`BETAS`].
///
/// # Errors
///
/// The first such value.
pub fn betas(header: &str) -> Result<(), String> {
    match header
        .split(',')
        .map(str::trim)
        .find(|beta| !BETAS.contains(beta))
    {
        Some(beta) => Err(format!(
            "the anthropic-beta {beta:?} is not one the proxy can price"
        )),
        None => Ok(()),
    }
}

/// A request judged: what is forwarded, and what it asks for.
#[derive(Debug)]
pub struct Judged {
    /// The parse judged, serialized again: the bytes forwarded.
    pub body: Vec<u8>,
    /// `None` for a token count, which is not billed.
    pub demand: Option<Demand>,
}

/// What a message request asks for, for its worst case
/// (`api_prices.rs` `Price::worst`).
#[derive(Debug, PartialEq, Eq)]
pub struct Demand {
    pub model: String,
    /// The most output tokens it may be billed for.
    pub max_tokens: u64,
    /// An upper bound on its input tokens: the forwarded body's bytes,
    /// as a token is at least one byte of the text it stands for (the
    /// blocks forwarded are text, `api_request_content.rs`), plus
    /// [`TOOL_PROMPT_TOKENS`] when `tools` is present.
    pub input_bound: u64,
}

/// `target` and `body` judged: the bytes to forward and what a message
/// asks for.
///
/// # Errors
///
/// A target not in [`TARGETS`], a body that is not a JSON object with no
/// repeated key, a field not in [`FIELDS`], a content block other than
/// text, a server tool, a cache write other than the five-minute one,
/// or no `model` or `max_tokens`, each named.
pub fn judged(target: &str, body: &[u8]) -> Result<Judged, String> {
    if !TARGETS.contains(&target) {
        return Err(format!(
            "the target {target:?} is not one the proxy forwards ({})",
            TARGETS.join(", ")
        ));
    }
    let value = parsed(body)?;
    let object = value
        .as_object()
        .ok_or("a request body that is not a JSON object")?;
    if let Some(field) = object.keys().find(|key| !FIELDS.contains(&key.as_str())) {
        return Err(format!(
            "the request field {field:?} is not one the proxy can price"
        ));
    }
    priced(&value)?;
    let body = serde_json::to_vec(&value)
        .map_err(|e| format!("the request could not be written again: {e}"))?;
    if target.starts_with("/v1/messages/count_tokens") {
        return Ok(Judged { body, demand: None });
    }
    let demand = demand(&value, body.len())?;
    Ok(Judged {
        body,
        demand: Some(demand),
    })
}

/// Refuses what the prices cannot express: a block other than text, a
/// server tool, a cache write other than the five-minute one.
fn priced(value: &Value) -> Result<(), String> {
    plain(value)?;
    server_tools(&value["tools"])?;
    cache_writes(value)
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
    let tools = value["tools"]
        .as_array()
        .is_some_and(|tools| !tools.is_empty());
    let prompt = if tools { TOOL_PROMPT_TOKENS } else { 0 };
    let input_bound = u64::try_from(bytes).map_err(|_| "a body too long to count")? + prompt;
    Ok(Demand {
        model,
        max_tokens,
        input_bound,
    })
}

#[cfg(test)]
#[path = "api_request_policy_tests.rs"]
mod tests;
