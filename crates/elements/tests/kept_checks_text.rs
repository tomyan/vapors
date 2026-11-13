//! The right-to-left text check is a text node's own, kept by its id
//! (`check.rs`): a text node inserted, changed or removed after an
//! update is checked again, and its parent is checked without reading
//! its children, so a parent whose children changed is checked in
//! constant time (`insert_checks_rows.rs`).

mod support;

use support::{document, element, refusals_at, text, viewport};
use vapors_dom::Dom;
use vapors_elements::Document;

/// Hebrew, which vapors does not lay out right to left yet.
const HEBREW: &str = "\u{05E9}\u{05DC}\u{05D5}\u{05DD}";

/// Asserts the update is refused with a message holding `says`. Debug
/// builds walk the tree on every update and panic when the kept checks
/// disagree, so the refusals are the ones a full check reports.
fn assert_refused(doc: &mut Document, says: &str) {
    let found = refusals_at(doc, viewport(100.0, 100.0));
    assert!(
        found.iter().any(|r| r.contains(says)),
        "{says} in {found:?}"
    );
}

fn assert_accepted(doc: &mut Document) {
    let updated = doc.update(viewport(100.0, 100.0));
    assert!(updated.is_ok(), "{:?}", updated.err());
}

#[test]
fn right_to_left_text_inserted_after_an_update_is_refused_until_it_is_removed() {
    // Given
    let mut doc = document();
    let body = doc.body();
    let p = element(&mut doc, body, "p", "greeting");
    assert_accepted(&mut doc);

    // When
    let words = text(&mut doc, p, HEBREW);
    assert_refused(&mut doc, "<p class=\"greeting\">: its text holds");
    doc.remove(words);

    // Then
    assert_accepted(&mut doc);
}

#[test]
fn an_element_inserted_with_right_to_left_text_below_it_is_refused() {
    // Given
    let mut doc = document();
    let body = doc.body();
    let section = element(&mut doc, body, "section", "");
    assert_accepted(&mut doc);

    // When: <div><p>HEBREW</p></div> goes under the section
    let div = doc.create_element("div");
    let p = element(&mut doc, div, "p", "greeting");
    text(&mut doc, p, HEBREW);
    doc.insert_before(section, div, None);

    // Then
    assert_refused(&mut doc, "<p class=\"greeting\">: its text holds");
}

#[test]
fn right_to_left_text_moved_to_another_element_names_that_element() {
    // Given: refused under one element
    let mut doc = document();
    let body = doc.body();
    let first = element(&mut doc, body, "p", "first");
    let second = element(&mut doc, body, "p", "second");
    let words = text(&mut doc, first, HEBREW);
    assert_refused(&mut doc, "<p class=\"first\">");

    // When
    doc.insert_before(second, words, None);

    // Then
    let found = refusals_at(&mut doc, viewport(100.0, 100.0));
    let names = |element: &str| found.iter().any(|r| r.contains(element));
    assert!(
        names("<p class=\"second\">") && !names("<p class=\"first\">"),
        "{found:?}"
    );
}
