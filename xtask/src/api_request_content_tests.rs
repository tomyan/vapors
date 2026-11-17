use serde_json::json;

use super::plain;

#[test]
fn text_tool_calls_results_and_thinking_are_forwarded() {
    let body = json!({"system": [{"type": "text", "text": "s"}], "messages": [
        {"role": "user", "content": "plain"},
        {"role": "assistant", "content": [{"type": "thinking", "thinking": ""}, {"type": "tool_use", "id": "1", "name": "Bash", "input": {}}]},
        {"role": "user", "content": [{"type": "tool_result", "tool_use_id": "1", "content": [{"type": "text", "text": "ok"}]}]}
    ]});
    assert_eq!(plain(&body), Ok(()));
}

#[test]
fn an_image_or_document_by_any_source_is_refused_even_inside_a_tool_result() {
    for block in [
        json!({"type": "image", "source": {"type": "url", "url": "https://x/a.png"}}),
        json!({"type": "document", "source": {"type": "base64", "media_type": "application/pdf", "data": "JVBERi0="}}),
        json!({"type": "tool_result", "tool_use_id": "1", "content": [{"type": "image", "source": {"type": "base64", "data": "x"}}]}),
    ] {
        let body = json!({"messages": [{"role": "user", "content": [block]}]});
        let refused = plain(&body).expect_err("refused");
        assert!(refused.starts_with("a content block of type"), "{refused}");
    }
}
