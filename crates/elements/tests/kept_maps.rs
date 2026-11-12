//! The `rects` and `paddings` maps kept across updates and patched only
//! for the changed slots (IU3 3c-3b, `docs/plans/iu3.md`): a rebuilt or
//! moved element's entries are replaced, a freed one's removed, and an
//! update that changes no slot patches none. A debug build also
//! compares the kept maps with freshly built ones on every update that
//! paints (`kept_paint_check.rs`).

mod support;

use support::{document, element, text, viewport};
use vapors_dom::{Dom, NodeId};
use vapors_elements::Document;

/// Fixed-size blocks in a fixed-size body, so an edit moves only what
/// it names.
const CSS: &str = "body { margin: 0; height: 100px; } \
    div { box-sizing: border-box; height: 10px; } \
    .tall { height: 30px; } .bordered { border: 3px solid black; } \
    .gone { display: none; } \
    .scroller { height: 20px; overflow: auto; }";

/// A document of two blocks, updated once.
fn two_blocks() -> (Document, NodeId, NodeId) {
    let mut doc = document();
    doc.add_stylesheet("app.css", CSS).expect("parses");
    let body = doc.body();
    let first = element(&mut doc, body, "div", "");
    let second = element(&mut doc, body, "div", "");
    updated(&mut doc);
    (doc, first, second)
}

fn updated(doc: &mut Document) {
    doc.update(viewport(200.0, 200.0)).expect("updates");
}

fn painted(doc: &Document) -> &vapors_elements::Painted {
    doc.painted().expect("the last update painted")
}

#[test]
fn a_moved_elements_entries_are_patched() {
    // Given
    let (mut doc, first, second) = two_blocks();

    // When: the first block grows, moving the second down
    doc.set_attribute(first, "class", "tall");
    updated(&mut doc);

    // Then: the first rebuilt and the second moved, nothing else
    let rect = painted(&doc).border_box(second).expect("has a box");
    assert_eq!(rect.y, 30.0);
    assert_eq!(painted(&doc).padding_origin(second), Some([0.0, 30.0]));
    assert_eq!(doc.update_counts().map_entries_patched, 2);
}

#[test]
fn a_freed_elements_entries_are_removed() {
    // Given
    let (mut doc, _, second) = two_blocks();

    // When
    doc.remove(second);
    updated(&mut doc);

    // Then: body rebuilt, the first block kept, the second freed
    assert_eq!(painted(&doc).border_box(second), None);
    assert_eq!(painted(&doc).padding_origin(second), None);
    assert_eq!(doc.update_counts().map_entries_patched, 2);
}

#[test]
fn a_rebuilt_elements_entries_are_replaced() {
    // Given
    let (mut doc, first, _) = two_blocks();

    // When: a border moves the padding box in, not the border box
    doc.set_attribute(first, "class", "bordered");
    updated(&mut doc);

    // Then
    assert_eq!(painted(&doc).padding_origin(first), Some([3.0, 3.0]));
    assert_eq!(doc.update_counts().map_entries_patched, 1);
}

#[test]
fn an_update_that_keeps_the_last_paint_patches_no_entry() {
    // Given
    let (mut doc, _, _) = two_blocks();

    // When: nothing changed, so the last paint is kept
    updated(&mut doc);

    // Then
    assert_eq!(doc.update_counts().map_entries_patched, 0);
}

#[test]
fn an_update_that_paints_but_changes_no_slot_patches_no_entry() {
    // Given
    let (mut doc, _, _) = two_blocks();

    // When: hiding the thumbs paints again and changes no box
    doc.set_thumbs_shown(false);
    updated(&mut doc);

    // Then
    assert_eq!(doc.update_counts().map_entries_patched, 0);
}

#[test]
fn a_scroll_patches_no_entry_and_the_children_read_where_they_show() {
    // Given: a scroller of five blocks, two in view
    let mut doc = document();
    doc.add_stylesheet("app.css", CSS).expect("parses");
    let body = doc.body();
    let scroller = element(&mut doc, body, "div", "scroller");
    let children: Vec<NodeId> = (0..5)
        .map(|_| element(&mut doc, scroller, "div", ""))
        .collect();
    updated(&mut doc);

    // When
    assert!(doc.scroll_to(scroller, [0.0, 10.0]), "scrolls");
    updated(&mut doc);

    // Then: the children's entries are in the scroller's frame, which
    // the scroll shifted (IU4, `frames.rs`), so none is patched
    let rect = painted(&doc).border_box(children[1]).expect("has a box");
    assert_eq!(rect.y, 0.0);
    assert_eq!(doc.update_counts().map_entries_patched, 0);
}

/// A paragraph holding a span and text, updated once.
fn paragraph_with_span() -> (Document, NodeId) {
    let mut doc = document();
    doc.add_stylesheet("app.css", CSS).expect("parses");
    let body = doc.body();
    let paragraph = element(&mut doc, body, "p", "");
    let span = element(&mut doc, paragraph, "span", "");
    text(&mut doc, span, "tap");
    text(&mut doc, paragraph, " ok");
    updated(&mut doc);
    assert!(
        painted(&doc).border_box(span).is_some(),
        "the span has a box"
    );
    (doc, span)
}

#[test]
fn a_span_hidden_from_its_run_loses_its_rect() {
    // Given
    let (mut doc, span) = paragraph_with_span();

    // When
    doc.set_attribute(span, "class", "gone");
    updated(&mut doc);

    // Then
    assert_eq!(painted(&doc).border_box(span), None);
}

#[test]
fn a_span_removed_from_its_run_loses_its_rect() {
    // Given
    let (mut doc, span) = paragraph_with_span();

    // When
    doc.remove(span);
    updated(&mut doc);

    // Then
    assert_eq!(painted(&doc).border_box(span), None);
}
