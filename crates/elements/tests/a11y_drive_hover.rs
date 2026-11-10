//! An action that happened is reported as done (R1), even when the
//! tree after it cannot be read: the reply says why the tree is
//! unavailable, so a driver does not retry the action. A command with
//! a field it does not have is refused naming the field.

mod support;

use support::drive::{log, node, shared, target};
use support::roles_page::page;
use vapors_elements::{AxError, Command, Document, Reply};

#[test]
fn a_press_whose_tree_hover_keeps_changing_is_done_without_a_tree() {
    // Given: hovering Mono moves it from under the pointer, and back
    let mut page = page();
    let shift = "input[type=checkbox]:hover { margin-left: 40px; }";
    page.add_stylesheet("hover.css", shift).expect("parses");
    page.update(support::viewport(640.0, 400.0))
        .expect("updates");
    let doc = shared(page);
    let mono = node(&doc, "checkbox", "Mono");
    let changes = log(&doc, mono, &["change"]);

    // When
    let reply = Document::drive(
        &doc,
        &Command::Press {
            target: target("checkbox", "Mono"),
        },
    );

    // Then: checked once, and the reply says so, with why there is no tree
    assert_eq!(*changes.borrow(), ["change"]);
    let written = serde_json::to_string(&reply).expect("serialises");
    let json: serde_json::Value = serde_json::from_str(&written).expect("JSON");
    assert_eq!(json["problem"], "not_updated", "one level: {written}");
    assert_eq!(
        serde_json::from_str::<Reply>(&written).expect("reads"),
        reply
    );
    let Reply::Done {
        nodes: None,
        problem: Some(AxError::NotUpdated),
        ..
    } = reply
    else {
        panic!("done without a tree expected, got {reply:?}")
    };
}

#[test]
fn a_command_with_a_field_it_does_not_have_is_refused_naming_it() {
    // Given: "modifier" for "modifiers", "withn" for "within"
    let misspelt = [
        (
            r#"{"command": "key", "target": {"role": "slider", "name": "Amount"}, "key": "a", "modifier": ["shift"]}"#,
            "modifier",
        ),
        (
            r#"{"command": "find", "target": {"role": "slider", "name": "Amount", "withn": {"role": "group", "name": "Tone"}}}"#,
            "withn",
        ),
        (
            r#"{"command": "find", "target": {"role": "slider", "name": "Amount", "within": {"role": "group", "name": "Tone", "x": 1}}}"#,
            "x",
        ),
    ];
    for (json, field) in misspelt {
        // When
        let parsed = serde_json::from_str::<Command>(json);

        // Then
        let error = parsed.expect_err(json).to_string();
        assert!(
            error.contains(&format!("unknown field `{field}`")),
            "{error}"
        );
    }
}
