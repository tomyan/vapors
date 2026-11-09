//! Finding one node of the a11y tree by role and exact name (R1),
//! within an ancestor's when the name alone is not enough: no match, or
//! more than one, is refused naming the role, the name and the
//! candidates, never answered with the first.

mod support;

use support::roles_page::{button, labelled, page};
use support::viewport;
use vapors_elements::{AxError, Document};

/// The roles page with a second Apply button, in a group named Extra.
fn two_applies() -> Document {
    let mut doc = page();
    let dialog = doc.children(doc.body())[0];
    let extra = labelled(
        &mut doc,
        dialog,
        "div",
        &[("class", "row"), ("role", "group"), ("aria-label", "Extra")],
    );
    button(&mut doc, extra, "Apply", &[]);
    doc.update(viewport(640.0, 400.0)).expect("updates");
    doc
}

#[test]
fn a_node_is_found_by_role_and_exact_name() {
    // Given
    let doc = page();

    // When
    let found = doc
        .ax_find("textbox", "Note", None)
        .expect("one textbox Note");

    // Then: the text field, with its value
    assert_eq!(doc.attribute(found.node, "id"), Some("note"));
    assert_eq!(found.value.as_deref(), Some("hi"));
}

#[test]
fn no_match_is_refused_naming_the_nodes_of_that_role() {
    // Given
    let doc = page();

    // When: a name that is only a prefix
    let refused = doc.ax_find("textbox", "No", None);

    // Then
    assert_eq!(
        refused.map(|_| ()),
        Err(AxError::Missing {
            role: "textbox".into(),
            name: "No".into(),
            candidates: vec!["dialog \"Levels\" > group \"Tone\" > textbox \"Note\"".into()],
        })
    );
}

#[test]
fn more_than_one_match_is_refused_naming_each_within_its_ancestors() {
    // Given
    let doc = two_applies();

    // When
    let refused = doc.ax_find("button", "Apply", None);

    // Then
    assert_eq!(
        refused.map(|_| ()),
        Err(AxError::Ambiguous {
            role: "button".into(),
            name: "Apply".into(),
            matches: vec![
                "dialog \"Levels\" > region \"Options\" > button \"Apply\"".into(),
                "dialog \"Levels\" > group \"Extra\" > button \"Apply\"".into(),
            ],
        })
    );
}

#[test]
fn an_ancestor_tells_two_matches_apart() {
    // Given
    let doc = two_applies();

    // When
    let found = doc
        .ax_find("button", "Apply", Some(("group", "Extra")))
        .expect("one Apply in Extra");

    // Then: the last button
    let extra = doc.children(doc.children(doc.body())[0])[3];
    assert_eq!(doc.parent(found.node), Some(extra));
}

#[test]
fn a_refusal_names_the_fix_and_the_candidates() {
    // Given
    let doc = two_applies();

    // When
    let message = doc
        .ax_find("button", "Apply", None)
        .map(|_| ())
        .unwrap_err()
        .to_string();

    // Then
    assert!(
        message.contains("2 nodes are button \"Apply\""),
        "{message}"
    );
    assert!(
        message.contains("group \"Extra\" > button \"Apply\""),
        "{message}"
    );
    assert!(message.contains("fix: name an ancestor"), "{message}");
}
