//! A placeholder in the a11y tree (`docs/placeholder.md`): the field's
//! description, never its name.

use crate::support;

use support::{document, element, viewport};
use vapors_dom::Dom;
use vapors_elements::Document;

/// A 100 px text field with `attributes`, updated.
fn field(attributes: &[(&str, &str)]) -> (Document, vapors_dom::NodeId) {
    let mut doc = document();
    let body = doc.body();
    let input = element(&mut doc, body, "input", "");
    for (name, value) in attributes {
        doc.set_attribute(input, name, value);
    }
    doc.add_stylesheet("a.css", "input { display: block; width: 100px; }")
        .expect("parses");
    doc.update(viewport(200.0, 100.0)).expect("updates");
    (doc, input)
}

fn textbox(doc: &Document) -> (String, Option<String>) {
    let tree = doc.ax_tree().expect("an updated document");
    let node = &tree[0];
    assert_eq!(node.role, "textbox");
    (node.name.clone(), node.description.clone())
}

#[test]
fn the_placeholder_is_the_description_of_a_named_field() {
    // Given
    let (doc, _) = field(&[("aria-label", "Search"), ("placeholder", "Find a report")]);

    // Then
    assert_eq!(
        textbox(&doc),
        ("Search".into(), Some("Find a report".into()))
    );
}

#[test]
fn the_placeholder_is_not_the_name_of_an_unnamed_field() {
    // Given
    let (doc, _) = field(&[("placeholder", "Find a report")]);

    // Then
    assert_eq!(textbox(&doc), (String::new(), Some("Find a report".into())));
}

#[test]
fn a_described_by_description_comes_before_the_placeholder_and_a_value_keeps_it() {
    // Given: a description by id, then a field with a value
    let (mut doc, input) = field(&[("aria-label", "Q"), ("placeholder", "hint"), ("value", "x")]);
    assert_eq!(textbox(&doc).1.as_deref(), Some("hint"));
    let note = doc.create_element("span");
    doc.set_attribute(note, "id", "note");
    let body = doc.body();
    doc.append(body, note);
    let words = doc.create_text("Longer help");
    doc.append(note, words);

    // When
    doc.set_attribute(input, "aria-describedby", "note");
    doc.update(viewport(200.0, 100.0)).expect("updates");

    // Then
    assert_eq!(textbox(&doc).1.as_deref(), Some("Longer help"));
}

#[test]
fn a_field_without_a_placeholder_has_no_description() {
    // Given
    let (doc, _) = field(&[("aria-label", "Search")]);

    // Then
    assert_eq!(textbox(&doc), ("Search".into(), None));
}
