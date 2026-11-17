use serde_json::json;

use super::answer;

/// A port nothing listens on: a call that reaches out fails at once.
const NOBODY: u16 = 9;

#[test]
fn initialize_answers_with_the_clients_protocol_version_and_tools() {
    // When
    let reply = answer(
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18"}}"#,
        NOBODY,
    );

    // Then
    let result = &reply.expect("a reply")["result"];
    assert_eq!(result["protocolVersion"], "2025-06-18");
    assert_eq!(result["capabilities"], json!({"tools": {}}));
}

#[test]
fn the_tools_listed_are_the_harnesss_with_their_argument_required() {
    let reply =
        answer(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#, NOBODY).expect("a reply");
    let tool = &reply["result"]["tools"][0];
    assert_eq!(tool["name"], "ask_lead");
    assert_eq!(tool["inputSchema"]["required"], json!(["question"]));
}

#[test]
fn a_notification_gets_no_reply() {
    assert_eq!(
        answer(
            r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
            NOBODY
        ),
        None
    );
}

#[test]
fn a_call_the_harness_cannot_answer_is_the_calls_error_not_a_crash() {
    // When
    let reply = answer(
        r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"ask_lead","arguments":{"question":"q"}}}"#,
        NOBODY,
    )
    .expect("a reply");

    // Then
    assert_eq!(reply["result"]["isError"], true);
    let text = reply["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_default();
    assert!(text.starts_with("the relay:"), "{text}");
}

#[test]
fn an_unknown_method_is_a_json_rpc_error() {
    let reply = answer(
        r#"{"jsonrpc":"2.0","id":4,"method":"resources/list"}"#,
        NOBODY,
    )
    .expect("a reply");
    assert_eq!(reply["error"]["code"], -32601);
}
