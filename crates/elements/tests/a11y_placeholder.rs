//! A placeholder in the a11y tree (`docs/placeholder.md`): the field's
//! description, never its name.

use crate::support;

use support::{document, element, viewport};
use vapors_dom::Dom;
use vapors_elements::Document;

/// A 100 px input with `attributes`, updated.
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

fn described(doc: &Document, role: &str) -> (String, Option<String>) {
    let tree = doc.ax_tree().expect("an updated document");
    let node = &tree[0];
    assert_eq!(node.role, role);
    (node.name.clone(), node.description.clone())
}

fn textbox(doc: &Document) -> (String, Option<String>) {
    described(doc, "textbox")
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
fn a_title_that_names_the_field_leaves_the_placeholder_its_description() {
    // Given: no other name, so the title is the name
    let (doc, _) = field(&[("title", "Report"), ("placeholder", "Find a report")]);

    // Then
    assert_eq!(
        textbox(&doc),
        ("Report".into(), Some("Find a report".into()))
    );
}

#[test]
fn a_title_that_describes_the_field_wins_over_the_placeholder() {
    // Given: the name is the aria-label's, so the title describes
    let (doc, _) = field(&[
        ("aria-label", "Search"),
        ("title", "Looks in reports"),
        ("placeholder", "Find a report"),
    ]);

    // Then
    assert_eq!(
        textbox(&doc),
        ("Search".into(), Some("Looks in reports".into()))
    );
}
