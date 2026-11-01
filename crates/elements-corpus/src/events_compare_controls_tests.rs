//! Comparing the controls a step names: values, checked states and
//! selections that differ are reported, and one either side lacks is
//! refused, not passed.

use serde_json::json;

use crate::events_compare::compare;
use crate::events_script::parse;
use crate::events_vapors::Logged;

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
fn a_checked_state_the_vapors_log_lacks_is_refused_not_passed() {
    // Given: the script expects a state, vapors recorded none
    let script = parse(
        "x",
        "listen click\nclick 5 5\nexpect checked input#a true\n",
    )
    .expect("parses");
    let ours = vec![Logged {
        focus: "body".to_owned(),
        ..Logged::default()
    }];
    let answers = json!({ "steps": [{ "line": 2, "action": "Click(5.0, 5.0)", "events": [], "focus": "body", "focusVisible": false, "values": {}, "checked": { "input#a": true } }] });

    // When
    let refused = compare("x", &script, &ours, &answers).expect_err("refused");

    // Then
    assert!(
        refused.contains("x.events:2: vapors' log has no checked state for input#a"),
        "{refused}"
    );
}

#[test]
fn a_checked_state_that_differs_is_reported_and_one_not_recorded_asks_for_a_rerun() {
    // Given: vapors says checked, Chromium not, then a recording without it
    let script = parse(
        "x",
        "listen click\nclick 5 5\nexpect checked input#a true\n",
    )
    .expect("parses");
    let ours = vec![Logged {
        checked: vec![("input#a".to_owned(), true)],
        focus: "body".to_owned(),
        ..Logged::default()
    }];
    let step = |checked: serde_json::Value| json!({ "steps": [{ "line": 2, "action": "Click(5.0, 5.0)", "events": [], "focus": "body", "focusVisible": false, "values": {}, "checked": checked }] });

    // When
    let differs =
        compare("x", &script, &ours, &step(json!({ "input#a": false }))).expect_err("differs");
    let missing = compare("x", &script, &ours, &step(json!({}))).expect_err("missing");

    // Then
    assert!(
        differs.contains("x.events:2: input#a is checked false in Chromium, true in vapors"),
        "{differs}"
    );
    assert!(missing.contains("rerun the events oracle"), "{missing}");
}

#[test]
fn a_selection_that_differs_is_reported_and_one_not_recorded_asks_for_a_rerun() {
    // Given: vapors says 4..4, Chromium 0..4, then a recording without it
    let script =
        parse("x", "listen input\nkey End\nexpect selection input#a 4 4\n").expect("parses");
    let ours = vec![Logged {
        selections: vec![("input#a".to_owned(), (4, 4))],
        focus: "body".to_owned(),
        ..Logged::default()
    }];
    let step = |selections: serde_json::Value| json!({ "steps": [{ "line": 2, "action": "Key(\"End\")", "events": [], "focus": "body", "focusVisible": false, "selections": selections }] });

    // When
    let differs =
        compare("x", &script, &ours, &step(json!({ "input#a": [0, 4] }))).expect_err("differs");
    let missing = compare("x", &script, &ours, &step(json!({}))).expect_err("missing");
    let malformed =
        compare("x", &script, &ours, &step(json!({ "input#a": [4] }))).expect_err("malformed");

    // Then
    assert!(
        differs.contains("x.events:2: input#a has selection 0..4 in Chromium, 4..4 in vapors"),
        "{differs}"
    );
    assert!(missing.contains("rerun the events oracle"), "{missing}");
    assert!(malformed.contains("rerun the events oracle"), "{malformed}");
    assert_eq!(
        compare("x", &script, &ours, &step(json!({ "input#a": [4, 4] }))),
        Ok(())
    );
}

#[test]
fn a_selection_the_vapors_log_lacks_is_refused_not_passed() {
    // Given: the script expects a selection, vapors recorded none
    let script =
        parse("x", "listen input\nkey End\nexpect selection input#a 4 4\n").expect("parses");
    let ours = vec![Logged {
        focus: "body".to_owned(),
        ..Logged::default()
    }];
    let answers = json!({ "steps": [{ "line": 2, "action": "Key(\"End\")", "events": [], "focus": "body", "focusVisible": false, "selections": { "input#a": [4, 4] } }] });

    // When
    let refused = compare("x", &script, &ours, &answers).expect_err("refused");

    // Then
    assert!(
        refused.contains("x.events:2: vapors' log has no selection for input#a"),
        "{refused}"
    );
}
