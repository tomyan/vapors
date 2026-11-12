//! The Responses API request: the prompt, each frame as a base64 PNG in
//! the order the prompt lists them, the output cap, and a JSON schema
//! that lets the model name only the judged items and the frames sent.

use serde_json::{Value, json};

use crate::ux_checklist::Kind;
use crate::ux_config::UxConfig;
use crate::ux_review::Prepared;

/// What a judgement may say.
pub const VERDICTS: [&str; 4] = ["pass", "concern", "fail", "cannot-judge"];

/// The schema the answer must follow: one judgement per judged item,
/// citing frames by the names sent.
pub fn schema(prepared: &Prepared) -> Value {
    let items: Vec<&str> = prepared
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
    let judgement = json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["item", "verdict", "severity", "frames", "note"],
        "properties": {
            "item": { "type": "string", "enum": items },
            "verdict": { "type": "string", "enum": VERDICTS },
            "severity": { "type": "integer", "enum": [1, 2, 3] },
            "frames": { "type": "array", "items": { "type": "string", "enum": frames } },
            "note": { "type": "string" }
        }
    });
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["judgements"],
        "properties": { "judgements": { "type": "array", "items": judgement } }
    })
}

/// The request body, with each frame read and encoded.
///
/// # Errors
/// A frame that cannot be read, naming it.
pub fn body(ux: &UxConfig, prepared: &Prepared) -> Result<Value, String> {
    let mut content = vec![json!({ "type": "input_text", "text": prepared.prompt })];
    for frame in prepared.viewports.iter().flat_map(|v| &v.frames) {
        let png =
            std::fs::read(&frame.path).map_err(|e| format!("{}: {e}", frame.path.display()))?;
        let url = format!("data:image/png;base64,{}", crate::ux_base64::encode(&png));
        content.push(json!({ "type": "input_image", "image_url": url, "detail": ux.image_detail }));
    }
    Ok(json!({
        "model": ux.model.name,
        "store": false,
        "max_output_tokens": ux.caps.output_tokens,
        "reasoning": { "effort": ux.reasoning },
        "input": [{ "role": "user", "content": content }],
        "text": { "format": { "type": "json_schema", "name": "ux_review", "strict": true, "schema": schema(prepared) } }
    }))
}

#[cfg(test)]
#[path = "ux_request_tests.rs"]
mod tests;
