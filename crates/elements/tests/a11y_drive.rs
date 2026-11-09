//! One Command/Reply vocabulary for driving a document (R1), as JSON
//! for stdio, a socket and a page call: a command in, a reply out, and a
//! refusal a reply naming the problem.

mod support;

use serde_json::{Value, json};
use support::drive::{node, shared, update};
use support::roles_page::page;
use vapors_elements::{Command, Document, Reply};

/// `command` as JSON, driven; the reply as JSON.
fn drive(doc: &std::rc::Rc<std::cell::RefCell<Document>>, command: Value) -> Value {
    let command: Command = serde_json::from_value(command).expect("a command");
    serde_json::to_value(Document::drive(doc, &command)).expect("serialises")
}

#[test]
fn a_press_by_role_and_name_replies_done() {
    // Given
    let doc = shared(page());

    // When
    let reply = drive(
        &doc,
        json!({"command": "press", "role": "checkbox", "name": "Mono"}),
    );
    update(&doc);

    // Then
    assert_eq!(reply, json!({"reply": "done"}));
    let mono = doc
        .borrow()
        .ax_find("checkbox", "Mono", None)
        .expect("found");
    assert_eq!(mono.states.checked, Some(true));
}

#[test]
fn a_key_carries_its_modifiers() {
    // Given
    let doc = shared(page());
    let amount = node(&doc, "slider", "Amount");

    // When: Shift steps a range by ten
    let key = json!({"command": "key", "role": "slider", "name": "Amount",
        "key": "ArrowRight", "modifiers": ["shift"]});
    let reply = drive(&doc, key);

    // Then
    assert_eq!(reply, json!({"reply": "done"}));
    assert_eq!(doc.borrow().value(amount).as_deref(), Some("60"));
}

#[test]
fn find_looks_only_within_the_ancestor_given() {
    // Given: Amount is in the group Tone, not the region Options
    let doc = shared(page());
    let find = |within: Value| json!({"command": "find", "role": "slider", "name": "Amount", "within": within});

    // When
    let in_tone = drive(&doc, find(json!({"role": "group", "name": "Tone"})));
    let in_options = drive(&doc, find(json!({"role": "region", "name": "Options"})));

    // Then
    assert_eq!(in_tone["node"]["value"], "50");
    assert_eq!(in_options["problem"]["problem"], "missing");
}

#[test]
fn a_refusal_replies_the_problem_and_its_message() {
    // Given
    let doc = shared(page());

    // When
    let reply = drive(
        &doc,
        json!({"command": "set_value", "role": "button", "name": "Reset", "value": "1"}),
    );

    // Then
    let disabled = json!({"problem": "disabled", "role": "button", "name": "Reset"});
    assert_eq!(reply["reply"], "refused");
    assert_eq!(reply["problem"], disabled);
    assert!(
        reply["message"]
            .as_str()
            .is_some_and(|m| m.contains("is disabled"))
    );
}

#[test]
fn the_tree_replied_reads_back_as_the_same_reply() {
    // Given
    let doc = shared(page());

    // When
    let reply = Document::drive(&doc, &Command::Query);
    let text = serde_json::to_string(&reply).expect("serialises");

    // Then
    let Reply::Tree { ref nodes } = reply else {
        panic!("a tree expected, got {reply:?}");
    };
    assert_eq!(nodes[0].role, "dialog");
    assert_eq!(serde_json::from_str::<Reply>(&text).expect("reads"), reply);
}
