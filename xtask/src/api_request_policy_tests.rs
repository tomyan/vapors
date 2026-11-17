use super::{Demand, judged};

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
    let demand = judged("/v1/messages?beta=true", &sent).expect("judged");

    // Then
    let bound = sent.len() as u64;
    assert_eq!(
        demand,
        Some(Demand {
            model: "claude-sonnet-5-5".into(),
            max_tokens: 1000,
            input_bound: bound
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
    assert_eq!(
        judged("/v1/messages/count_tokens?beta=true", sent),
        Ok(None)
    );
}
