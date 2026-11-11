//! An element in a `display: none` subtree generates no box (CSS
//! Display 3 section 2.5), so the checks after styling, which concern
//! layout and painting, skip it; showing the subtree again checks it.

mod support;

use support::{document, element, refusals_at, text, viewport};
use vapors_dom::{Dom, NodeId};
use vapors_elements::Document;

const CSS: &str = "body { font-size: 16px; } .row { display: flex; } .hidden { display: none; } \
    .key { font-size: 13px; } .pad { padding: 2px; }";

/// A `div` of class `row_class` holding a 13px `span.key`: blockified in
/// a flex row, inline (and refused for its font-size) in a block.
fn row(row_class: &str) -> (Document, NodeId) {
    let mut doc = document();
    let body = doc.body();
    let row = element(&mut doc, body, "div", row_class);
    let key = element(&mut doc, row, "span", "key");
    text(&mut doc, key, "K");
    doc.add_stylesheet("app.css", CSS).expect("parses");
    (doc, row)
}

fn assert_accepted(doc: &mut Document) {
    let updated = doc.update(viewport(100.0, 100.0));
    assert!(updated.is_ok(), "{:?}", updated.err());
}

fn assert_refused(doc: &mut Document, says: &str) {
    let found = refusals_at(doc, viewport(100.0, 100.0));
    assert!(
        found.iter().any(|r| r.contains(says)),
        "{says} in {found:?}"
    );
}

#[test]
fn hiding_a_flex_row_does_not_refuse_its_child_that_would_be_inline() {
    // Given
    let (mut doc, row) = row("row");
    assert_accepted(&mut doc);

    // When
    doc.set_attribute(row, "class", "row hidden");

    // Then
    assert_accepted(&mut doc);
}

#[test]
fn a_subtree_hidden_from_the_first_update_is_not_refused() {
    // Given
    let (mut doc, _) = row("hidden");

    // Then
    assert_accepted(&mut doc);
}

#[test]
fn showing_the_row_as_a_block_refuses_its_inline_child() {
    // Given
    let (mut doc, row) = row("hidden");
    assert_accepted(&mut doc);

    // When
    doc.set_attribute(row, "class", "");

    // Then
    assert_refused(&mut doc, "font-size");
}

#[test]
fn showing_the_row_as_a_flex_row_again_is_accepted() {
    // Given
    let (mut doc, row) = row("row hidden");
    assert_accepted(&mut doc);

    // When
    doc.set_attribute(row, "class", "row");

    // Then
    assert_accepted(&mut doc);
}

#[test]
fn a_style_refusal_inside_a_hidden_subtree_waits_until_it_is_shown() {
    // Given
    let (mut doc, row) = row("hidden");
    let padded = element(&mut doc, row, "span", "pad");
    text(&mut doc, padded, "P");
    assert_accepted(&mut doc);

    // When
    doc.set_attribute(row, "class", "");

    // Then
    assert_refused(&mut doc, "padding");
}
