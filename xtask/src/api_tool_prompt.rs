//! The system prompt the API adds when a request has `tools`, in tokens:
//! not in the body's bytes, so added to the input bound (review of
//! #476-#478, N1; lead, L35). From the "Tool use system prompt tokens"
//! table in the Pricing section of
//! <https://platform.claude.com/docs/en/agents-and-tools/tool-use/overview>,
//! read 2026-11-17. Each model's figure is the larger of its two columns
//! (`auto`/`none` and `any`/`tool`); where the page leaves a column
//! blank, [`LARGEST`] stands in for it.

/// The largest figure on the page: Claude Opus 4.7, `any`/`tool`.
pub const LARGEST: u64 = 804;

/// Model, `auto`/`none`, `any`/`tool` (`None` where the page is blank).
const TABLE: [(&str, u64, Option<u64>); 5] = [
    ("claude-opus-5-5", 286, None),
    ("claude-opus-5", 286, Some(406)),
    ("claude-sonnet-5-5", 286, None),
    ("claude-sonnet-5", 354, Some(474)),
    ("claude-haiku-5-5", 286, Some(406)),
];

/// The most tokens of tool-use system prompt `model` can be billed: its
/// row's larger column, or [`LARGEST`] for a blank or a model not in the
/// table.
pub fn tool_prompt_tokens(model: &str) -> u64 {
    TABLE
        .iter()
        .find(|(name, _, _)| *name == model)
        .map_or(LARGEST, |&(_, auto, any)| auto.max(any.unwrap_or(LARGEST)))
}

#[cfg(test)]
#[path = "api_tool_prompt_tests.rs"]
mod tests;
