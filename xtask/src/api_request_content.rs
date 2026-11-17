//! What a request's messages may hold (review of #476-#478, N1): the
//! blocks a builder sends, text, tool calls and their results, and
//! thinking, each of which is no more tokens than it has bytes. An image
//! or a document is not: a URL source, or a small compressed file, can
//! stand for far more tokens than its bytes, so the worst case
//! (`api_prices.rs`) would not bound it. Every other block type is
//! refused.

use serde_json::Value;

/// The content block types forwarded.
const BLOCKS: [&str; 5] = [
    "text",
    "tool_use",
    "tool_result",
    "thinking",
    "redacted_thinking",
];

/// Refuses any block in `messages` or `system` of a type not in
/// [`BLOCKS`], naming it.
///
/// # Errors
///
/// That block's type.
pub fn plain(body: &Value) -> Result<(), String> {
    let messages = body["messages"].as_array().into_iter().flatten();
    let contents = messages
        .map(|message| &message["content"])
        .chain([&body["system"]]);
    contents.into_iter().try_for_each(blocks)
}

/// The blocks of one `content` (a string is text), and those inside a
/// tool result.
fn blocks(content: &Value) -> Result<(), String> {
    for block in content.as_array().into_iter().flatten() {
        let kind = block["type"].as_str().unwrap_or_default();
        if !BLOCKS.contains(&kind) {
            return Err(format!(
                "a content block of type {kind:?}: the proxy forwards text, tool calls, tool \
                 results and thinking only, whose tokens its worst case bounds"
            ));
        }
        blocks(&block["content"])?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "api_request_content_tests.rs"]
mod tests;
