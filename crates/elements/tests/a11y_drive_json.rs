//! A command as JSON text (R1), as stdio, the socket and the page call
//! carry it: text that is no command is refused as `bad_command`,
//! naming why, and a reply is JSON with its problem at the top level.

mod support;

use serde_json::Value;
use support::drive::shared;
use support::roles_page::page;
use vapors_elements::{AxError, Document, Reply};

/// `text` driven; the reply as JSON.
fn drive_text(text: &str) -> Value {
    let doc = shared(page());
    let reply = Document::drive_json_text(&doc, text);
    serde_json::from_str(&reply).expect("the reply is JSON")
}

#[test]
fn text_that_is_no_json_is_refused_naming_the_parse_error() {
    // Given
    let doc = shared(page());

    // When: cut short
    let reply = Document::drive_json(&doc, r#"{"command": "press","#);

    // Then
    let Reply::Refused {
        problem: AxError::BadCommand { error },
        message,
    } = reply
    else {
        panic!("bad_command expected, got {reply:?}");
    };
    assert!(error.contains("EOF while parsing"), "{error}");
    assert!(message.contains(&error), "{message}");
}

#[test]
fn a_command_vapors_does_not_have_is_refused_naming_it() {
    // When
    let reply = drive_text(r#"{"command": "jump"}"#);

    // Then
    assert_eq!(reply["reply"], "refused");
    assert_eq!(reply["problem"], "bad_command");
    let message = reply["message"].as_str().expect("a message");
    assert!(message.contains("unknown variant `jump`"), "{message}");
}

#[test]
fn a_query_as_text_replies_the_tree_as_text() {
    // When
    let reply = drive_text(r#"{"command": "query"}"#);

    // Then
    assert_eq!(reply["reply"], "tree");
    assert_eq!(reply["nodes"][0]["role"], "dialog");
}

#[test]
fn a_refusal_reads_back_as_the_same_reply() {
    // Given
    let doc = shared(page());
    let text = r#"{"command": "find", "target": {"role": "button", "name": "Nope"}}"#;

    // When
    let reply = Document::drive_json(&doc, text);
    let written = serde_json::to_string(&reply).expect("serialises");

    // Then
    assert_eq!(
        serde_json::from_str::<Reply>(&written).expect("reads"),
        reply
    );
}
