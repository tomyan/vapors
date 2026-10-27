//! Comparing vapors' log of a script with Chromium's recording: the
//! recording must be of this script, and of a shape it recognises.

use serde_json::json;

use super::*;
use crate::events_script::parse;
use crate::events_vapors::Logged;

const SCRIPT: &str = "listen click\nclick 5 5\nexpect event click div#a\n";

fn ours() -> Log {
    vec![Logged {
        events: vec![("click".to_owned(), "div#a".to_owned())],
        focus: "body".to_owned(),
        focus_visible: false,
        values: Vec::new(),
    }]
}

fn recorded(events: &serde_json::Value) -> serde_json::Value {
    json!({ "steps": [{ "line": 2, "action": "Click(5.0, 5.0)", "events": events, "focus": "body", "focusVisible": false }] })
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
fn a_value_that_differs_is_reported_and_one_not_recorded_asks_for_a_rerun() {
    // Given: vapors says 50, Chromium 7, then a recording without values
    let script = parse("x", "listen input\nkey End\nexpect value input#a 50\n").expect("parses");
    let ours = vec![Logged {
        values: vec![("input#a".to_owned(), "50".to_owned())],
        focus: "body".to_owned(),
        ..Logged::default()
    }];
    let step = |values: serde_json::Value| json!({ "steps": [{ "line": 2, "action": "Key(\"End\")", "events": [], "focus": "body", "focusVisible": false, "values": values }] });

    // When
    let differs =
        compare("x", &script, &ours, &step(json!({ "input#a": "7" }))).expect_err("differs");
    let missing = compare("x", &script, &ours, &step(json!({}))).expect_err("missing");

    // Then
    assert!(
        differs.contains("x.events:2: input#a has value \"7\" in Chromium, \"50\" in vapors"),
        "{differs}"
    );
    assert!(missing.contains("rerun the events oracle"), "{missing}");
    assert_eq!(
        compare("x", &script, &ours, &step(json!({ "input#a": "50" }))),
        Ok(())
    );
}

#[test]
fn a_value_the_vapors_log_lacks_is_refused_not_passed() {
    // Given: the script expects a value, vapors recorded none
    let script = parse("x", "listen input\nkey End\nexpect value input#a 50\n").expect("parses");
    let ours = vec![Logged {
        focus: "body".to_owned(),
        ..Logged::default()
    }];
    let answers = json!({ "steps": [{ "line": 2, "action": "Key(\"End\")", "events": [], "focus": "body", "focusVisible": false, "values": { "input#a": "50" } }] });

    // When
    let refused = compare("x", &script, &ours, &answers).expect_err("refused");

    // Then
    assert!(
        refused.contains("x.events:2: vapors' log has no value for input#a"),
        "{refused}"
    );
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

#[test]
fn a_different_focus_is_a_difference() {
    // Given: Chromium has focus on the button, vapors on nothing
    let script = parse("x", SCRIPT).expect("parses");
    let mut answers = recorded(&json!([{ "type": "click", "target": "div#a" }]));
    answers["steps"][0]["focus"] = json!("button#a");

    // When
    let refused = compare("x", &script, &ours(), &answers).expect_err("differs");

    // Then
    assert!(
        refused.contains("focus is on button#a in Chromium, body in vapors"),
        "{refused}"
    );
}

#[test]
fn a_different_focus_visible_is_a_difference() {
    // Given: Chromium shows the ring, vapors does not
    let script = parse("x", SCRIPT).expect("parses");
    let mut answers = recorded(&json!([{ "type": "click", "target": "div#a" }]));
    answers["steps"][0]["focusVisible"] = json!(true);

    // When
    let refused = compare("x", &script, &ours(), &answers).expect_err("differs");

    // Then
    assert!(
        refused.contains(":focus-visible is true in Chromium, false in vapors"),
        "{refused}"
    );
}
