use super::parsed;

#[test]
fn a_body_with_no_repeated_key_parses_as_serde_json_would() {
    let body = br#"{"model":"m","max_tokens":10,"messages":[{"role":"user","content":[{"type":"text","text":"x"}]}]}"#;
    assert_eq!(
        parsed(body),
        Ok(serde_json::from_slice(body).expect("json"))
    );
}

#[test]
fn a_key_repeated_in_one_object_is_refused_at_any_depth() {
    for body in [
        &br#"{"max_tokens":128000,"model":"m","max_tokens":1}"#[..],
        br#"{"messages":[{"content":[{"cache_control":{"type":"ephemeral","ttl":"5m","ttl":"1h"}}]}]}"#,
    ] {
        let refused = parsed(body).expect_err("refused");
        assert!(refused.contains("appears twice in one object"), "{refused}");
    }
}

#[test]
fn trailing_bytes_after_the_json_are_refused() {
    assert!(parsed(br#"{"a":1} {"b":2}"#).is_err());
}
