//! The `_if_changed` setters on the document (IU2): a text or attribute
//! the node already holds leaves the document unchanged, so the next
//! update restyles, lays out and shapes nothing; a different value is
//! written and drawn.

mod support;

use support::{document, element, text, viewport};
use vapors_dom::{Dom, NodeId};
use vapors_elements::Document;

const SHEET: &str =
    "body { margin: 0; font-size: 14px; } .wide { width: 200px; } .small { width: 100px; }";

/// A settled document: a `.wide` paragraph holding the text "72"; the
/// change flag taken.
fn settled() -> (Document, NodeId, NodeId) {
    let mut doc = document();
    let body = doc.body();
    let paragraph = element(&mut doc, body, "p", "wide");
    let speed = text(&mut doc, paragraph, "72");
    doc.add_stylesheet("app.css", SHEET).expect("parses");
    doc.update(viewport(400.0, 100.0)).expect("updates");
    doc.take_changed();
    (doc, paragraph, speed)
}

/// Elements restyled, boxes laid out and text layouts built by an update.
fn work(doc: &mut Document) -> (usize, usize, usize) {
    doc.update(viewport(400.0, 100.0)).expect("updates");
    (
        doc.elements_restyled(),
        doc.boxes_laid_out(),
        doc.text_layouts_built(),
    )
}

#[test]
fn an_equal_text_leaves_the_document_unchanged() {
    // Given
    let (mut doc, _, speed) = settled();

    // When
    doc.set_text_if_changed(speed, "72");

    // Then
    assert!(!doc.take_changed());
    assert_eq!(work(&mut doc), (0, 0, 0));
}

#[test]
fn an_equal_attribute_leaves_the_document_unchanged() {
    // Given
    let (mut doc, paragraph, _) = settled();

    // When
    doc.set_attribute_if_changed(paragraph, "class", "wide");

    // Then
    assert!(!doc.take_changed());
    assert_eq!(work(&mut doc), (0, 0, 0));
}

#[test]
fn a_different_text_is_written_and_shaped() {
    // Given
    let (mut doc, _, speed) = settled();

    // When
    doc.set_text_if_changed(speed, "81");

    // Then
    assert!(doc.take_changed());
    assert_eq!(doc.text(speed), Some("81"));
    assert_eq!(work(&mut doc).2, 1);
}

#[test]
fn a_different_attribute_is_written_and_restyled() {
    // Given
    let (mut doc, paragraph, _) = settled();

    // When
    doc.set_attribute_if_changed(paragraph, "class", "small");

    // Then
    assert!(doc.take_changed());
    assert!(work(&mut doc).0 >= 1);
}
