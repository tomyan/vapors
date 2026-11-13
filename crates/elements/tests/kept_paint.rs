//! The last update's paint used again (IU3 3c-1): an update paints no
//! box when none was built and every box's rect, the thumbs and the
//! scale are those the kept paint shows. Each of those, changed alone,
//! paints again: the boxes built, and the thumbs; the moved are shifted.

mod support;

use support::scroll_list::{at, list, scroll};
use support::{document, element, text, viewport};
use vapors_dom::Dom;
use vapors_elements::{Document, UpdateCounts, Viewport};

/// `html` > `body` > a div with text: 4 boxes; returns the text node.
fn page() -> (Document, vapors_dom::NodeId) {
    let mut doc = document();
    let body = doc.body();
    let label = element(&mut doc, body, "div", "");
    let run = text(&mut doc, label, "label");
    (doc, run)
}

/// A page with no box: html is `display: none`.
fn no_boxes() -> Document {
    let mut doc = document();
    doc.add_stylesheet("app.css", ".hidden { display: none; }")
        .expect("parses");
    let root = doc.root();
    doc.set_attribute(root, "class", "hidden");
    doc
}

fn at_scale(scale: f32) -> Viewport {
    Viewport {
        width: 100.0,
        height: 100.0,
        scale,
    }
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
        .to_scene()
        .clone();

    // When
    let again = doc
        .update(viewport(100.0, 100.0))
        .expect("updates")
        .scene()
        .to_scene()
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

    // Then: the div and its text
    assert_eq!(work(doc.update_counts()), (2, 0, 2));
}

#[test]
fn a_scroll_alone_paints_and_shifts_no_row() {
    // Given: html, body, the list and its 4 rows
    let mut list = list("overflow: auto;");

    // When
    assert!(scroll(&mut list.doc, [0.0, 30.0]));
    list.doc.update(at(1.0)).expect("updates");

    // Then: the rows are kept in the list's scroll frame, which the
    // scroll shifted (IU4, `frames.rs`)
    assert_eq!(work(list.doc.update_counts()), (0, 0, 0));
    assert_eq!(list.doc.update_counts().boxes_shifted, 0);
}

#[test]
fn hiding_the_thumbs_alone_paints_without_them() {
    // Given
    let mut list = list("overflow: auto;");
    let shown = items(&mut list.doc);

    // When
    list.doc.set_thumbs_shown(false);
    let hidden = items(&mut list.doc);

    // Then: no box painted, and the thumb's item gone
    assert_eq!(work(list.doc.update_counts()), (0, 0, 0));
    assert!(hidden < shown, "{hidden} items hidden, {shown} shown");
}

/// How many items an update at scale 1 shows.
fn items(doc: &mut Document) -> usize {
    doc.update(at(1.0))
        .expect("updates")
        .scene()
        .to_scene()
        .items()
        .len()
}

#[test]
fn a_scale_change_alone_paints_again() {
    // Given
    let (mut doc, _) = page();
    doc.update(at_scale(1.0)).expect("updates");

    // When
    let scale = doc.update(at_scale(2.0)).expect("updates").scale();

    // Then
    assert_eq!(work(doc.update_counts()), (4, 4, 4));
    assert!((scale - 2.0).abs() < f32::EPSILON, "scale {scale}");
}

#[test]
fn a_scale_change_alone_paints_again_with_no_box() {
    // Given: no box is built at either scale, and both have no rects
    let mut doc = no_boxes();
    doc.update(at_scale(1.0)).expect("updates");

    // When
    let scale = doc.update(at_scale(2.0)).expect("updates").scale();

    // Then
    assert!((scale - 2.0).abs() < f32::EPSILON, "scale {scale}");
}

#[test]
fn forgetting_what_is_kept_forgets_the_paint() {
    // Given: no box, so the next update builds none
    let mut doc = no_boxes();
    doc.update(at_scale(1.0)).expect("updates");

    // When
    doc.forget_kept();

    // Then
    assert!(doc.painted().is_none());
}
