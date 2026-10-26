//! Comparing vapors' log of a script with Chromium's recording: the
//! recording must be of this script, and of a shape it recognises.

use serde_json::json;

use super::*;
use crate::events_script::parse;

const SCRIPT: &str = "listen click\nclick 5 5\nexpect event click div#a\n";

fn ours() -> Log {
    vec![vec![("click".to_owned(), "div#a".to_owned())]]
}

fn recorded(events: &serde_json::Value) -> serde_json::Value {
    json!({ "steps": [{ "line": 2, "action": "Click(5.0, 5.0)", "events": events }] })
}

#[test]
fn a_matching_recording_compares_equal() {
    // Given
    let script = parse("x", SCRIPT).expect("parses");
    let answers = recorded(&json!([{ "type": "click", "target": "div#a" }]));

    // Then
    assert_eq!(compare("x", &script, &ours(), &answers), Ok(()));
}

#[test]
fn a_step_whose_events_are_not_a_list_is_refused_naming_its_line() {
    // Given
    let script = parse("x", SCRIPT).expect("parses");

    // When
    let refused =
        compare("x", &script, &ours(), &recorded(&json!("not a list"))).expect_err("refused");

    // Then
    assert!(
        refused.contains("x.events.json")
            && refused.contains("line 2")
            && refused.contains("events"),
        "{refused}"
    );
}

#[test]
fn an_event_without_a_type_or_target_is_refused() {
    // Given
    let script = parse("x", SCRIPT).expect("parses");

    // When
    let refused = compare(
        "x",
        &script,
        &ours(),
        &recorded(&json!([{ "target": "div#a" }])),
    )
    .expect_err("refused");

    // Then
    assert!(
        refused.contains("line 2") && refused.contains("type"),
        "{refused}"
    );
}

#[test]
fn a_recording_of_another_script_is_refused_before_any_log_is_compared() {
    // Given: the recording's step is a touch at line 99
    let script = parse("x", SCRIPT).expect("parses");
    let answers = json!({ "steps": [{ "line": 99, "action": "TouchDown(1.0, 1.0)", "events": [{ "type": "click", "target": "div#a" }] }] });

    // When
    let refused = compare("x", &script, &ours(), &answers).expect_err("refused");

    // Then
    assert!(refused.contains("rerun the events oracle"), "{refused}");
}
