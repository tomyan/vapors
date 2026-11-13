//! A node moved out of the root's tree into a detached parent, not
//! removed: nothing kept for it stays (`Document::mark_moved_out`), so a
//! refusal it carried does not refuse the next update as unmarked
//! (`style_checks.rs`, `walked`).

mod support;

use support::{document, element, refusals_at, text, viewport};
use vapors_dom::Dom;
use vapors_elements::Document;

/// Hebrew, which vapors does not lay out right to left yet.
const HEBREW: &str = "\u{05E9}\u{05DC}\u{05D5}\u{05DD}";

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
fn refused_text_moved_into_a_detached_parent_lets_the_update_through() {
    // Given: a text node the update refuses
    let mut doc = document();
    let body = doc.body();
    let p = element(&mut doc, body, "p", "greeting");
    let words = text(&mut doc, p, HEBREW);
    assert_refused(&mut doc, "right to left");

    // When: moved, not removed, into an element outside the tree
    let holder = doc.create_element("div");
    doc.insert_before(holder, words, None);

    // Then
    assert_accepted(&mut doc);
}

#[test]
fn a_refused_element_moved_into_a_detached_parent_lets_the_update_through() {
    // Given: an element with a tag the tree refuses
    let mut doc = document();
    let body = doc.body();
    let canvas = element(&mut doc, body, "canvas", "");
    assert_refused(&mut doc, "canvas");

    // When
    let holder = doc.create_element("div");
    doc.insert_before(holder, canvas, None);

    // Then
    assert_accepted(&mut doc);
}

#[test]
fn a_subtree_moved_into_a_detached_parent_and_back_is_refused_again() {
    // Given: refused text under a div, moved out of the tree
    let mut doc = document();
    let body = doc.body();
    let div = element(&mut doc, body, "div", "");
    let p = element(&mut doc, div, "p", "greeting");
    text(&mut doc, p, HEBREW);
    assert_refused(&mut doc, "right to left");
    let holder = doc.create_element("div");
    doc.insert_before(holder, div, None);
    assert_accepted(&mut doc);

    // When: moved back
    doc.insert_before(body, div, None);

    // Then
    assert_refused(&mut doc, "<p class=\"greeting\">: its text holds");
}
