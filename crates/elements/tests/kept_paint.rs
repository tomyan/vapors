//! The last update's paint used again (IU3 3c-1): an update paints no
//! box when none was built and every box's rect and the thumbs are
//! those the kept paint shows. Each of those, changed alone, paints
//! every box again.

mod support;

use support::scroll_list::{at, list, scroll};
use support::{document, element, text, viewport};
use vapors_dom::Dom;
use vapors_elements::{Document, UpdateCounts};

/// `html` > `body` > a div with text: 4 boxes; returns the text node.
fn page() -> (Document, vapors_dom::NodeId) {
    let mut doc = document();
    let body = doc.body();
    let label = element(&mut doc, body, "div", "");
    let run = text(&mut doc, label, "label");
    (doc, run)
}

/// Boxes built, boxes laid out, boxes painted.
fn work(counts: UpdateCounts) -> (usize, usize, usize) {
    (
        counts.boxes_built,
        counts.boxes_laid_out,
        counts.boxes_painted,
    )
}

#[test]
fn an_update_with_nothing_changed_shows_the_kept_scene() {
    // Given
    let (mut doc, _) = page();
    let first = doc
        .update(viewport(100.0, 100.0))
        .expect("updates")
        .scene()
        .clone();

    // When
    let again = doc
        .update(viewport(100.0, 100.0))
        .expect("updates")
        .scene()
        .clone();

    // Then
    assert_eq!(work(doc.update_counts()), (0, 0, 0));
    assert_eq!(again, first);
}

#[test]
fn a_built_box_paints_again() {
    // Given
    let (mut doc, run) = page();
    doc.add_stylesheet("app.css", ".on { background-color: red; }")
        .expect("parses");
    doc.update(viewport(100.0, 100.0)).expect("updates");
    let label = doc.parent(run).expect("a parent");

    // When: a background, which lays nothing out again
    doc.set_attribute(label, "class", "on");
    doc.update(viewport(100.0, 100.0)).expect("updates");

    // Then
    assert_eq!(work(doc.update_counts()), (2, 0, 4));
}

#[test]
fn a_scroll_alone_paints_again() {
    // Given: html, body, the list and its 4 rows
    let mut list = list("overflow: auto;");

    // When
    assert!(scroll(&mut list.doc, [0.0, 30.0]));
    list.doc.update(at(1.0)).expect("updates");

    // Then
    assert_eq!(work(list.doc.update_counts()), (0, 0, 7));
}

#[test]
fn hiding_the_thumbs_alone_paints_again() {
    // Given
    let mut list = list("overflow: auto;");
    list.doc.update(at(1.0)).expect("updates");

    // When
    list.doc.set_thumbs_shown(false);
    list.doc.update(at(1.0)).expect("updates");

    // Then
    assert_eq!(work(list.doc.update_counts()), (0, 0, 7));
}
