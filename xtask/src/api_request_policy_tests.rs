use super::{Demand, TOOL_PROMPT_TOKENS, betas, judged};

fn body(extra: &str) -> Vec<u8> {
    format!(r#"{{"model":"claude-sonnet-5-5","max_tokens":1000,"messages":[]{extra}}}"#)
        .into_bytes()
}

#[test]
fn a_message_as_the_cli_sends_it_is_judged_with_its_demand() {
    // Given: the CLI's shape, its cache writes five-minute ones
    let sent = body(
        r#","system":[{"type":"text","text":"x","cache_control":{"type":"ephemeral"}}],"stream":true"#,
    );

    // When
    let judged = judged("/v1/messages?beta=true", &sent).expect("judged");

    // Then: the bytes forwarded are the parse judged, and bound the input
    assert_eq!(judged.body, sent);
    assert_eq!(
        judged.demand,
        Some(Demand {
            model: "claude-sonnet-5-5".into(),
            max_tokens: 1000,
            input_bound: sent.len() as u64
        })
    );
}

#[test]
fn a_target_curl_could_expand_or_any_other_query_is_refused() {
    for target in [
        "/v1/messages?n=[1-500]",
        "/v1/messages?beta=true&x=1",
        "/v1/messages/batches",
        "/v1/models",
    ] {
        let refused = judged(target, &body("")).expect_err(target);
        assert!(refused.starts_with("the target"), "{target}: {refused}");
    }
}

#[test]
fn a_field_the_prices_cannot_express_is_refused_naming_it() {
    for (extra, named) in [
        (r#","speed":"fast""#, "speed"),
        (r#","inference_geo":"us""#, "inference_geo"),
    ] {
        let refused = judged("/v1/messages", &body(extra)).expect_err(named);
        assert!(refused.contains(&format!("{named:?}")), "{refused}");
    }
}

#[test]
fn an_hour_long_cache_write_is_refused() {
    let sent = body(
        r#","system":[{"type":"text","text":"x","cache_control":{"type":"ephemeral","ttl":"1h"}}]"#,
    );
    let refused = judged("/v1/messages", &sent).expect_err("refused");
    assert!(
        refused.contains("not the five-minute cache write"),
        "{refused}"
    );
}

#[test]
fn a_server_tool_is_refused() {
    let sent = body(r#","tools":[{"type":"web_search_20260209","name":"web_search"}]"#);
    let refused = judged("/v1/messages", &sent).expect_err("refused");
    assert!(refused.contains("is billed per use"), "{refused}");
}

#[test]
fn a_token_count_is_judged_but_not_billed() {
    let sent = br#"{"model":"claude-sonnet-5-5","messages":[]}"#;
    let judged = judged("/v1/messages/count_tokens?beta=true", sent).expect("judged");
    assert_eq!(judged.demand, None);
}

#[test]
fn tools_add_the_largest_tool_use_system_prompt_to_the_input_bound() {
    let sent = body(r#","tools":[{"name":"Bash","input_schema":{}}]"#);
    let demand = judged("/v1/messages", &sent)
        .expect("judged")
        .demand
        .expect("demand");
    assert_eq!(demand.input_bound, sent.len() as u64 + TOOL_PROMPT_TOKENS);
}

#[test]
fn a_repeated_key_is_refused_so_the_max_tokens_judged_is_the_one_sent() {
    let sent = body(r#","max_tokens":1"#);
    let refused = judged("/v1/messages", &sent).expect_err("refused");
    assert!(
        refused.contains("\"max_tokens\" appears twice"),
        "{refused}"
    );
}

#[test]
fn an_image_by_url_is_refused_as_its_tokens_are_not_bounded_by_its_bytes() {
    let image = r#"{"type":"image","source":{"type":"url","url":"https://x/a.png"}}"#;
    let sent = format!(
        r#"{{"model":"m","max_tokens":1,"messages":[{{"role":"user","content":[{image}]}}]}}"#
    );
    let refused = judged("/v1/messages", sent.as_bytes()).expect_err("refused");
    assert!(refused.contains("\"image\""), "{refused}");
}

#[test]
fn the_betas_the_cli_sends_are_forwarded_and_any_other_is_refused_naming_it() {
    let sent = "claude-code-20250219, interleaved-thinking-2025-05-14,context-management-2025-06-27,per-turn-control-2026-07-01,advisor-tool-2026-03-01,thinking-token-count-2026-05-13,effort-2025-11-24";
    assert_eq!(betas(sent), Ok(()));
    let refused = betas("claude-code-20250219,context-1m-2025-08-07").expect_err("refused");
    assert!(refused.contains("\"context-1m-2025-08-07\""), "{refused}");
}
