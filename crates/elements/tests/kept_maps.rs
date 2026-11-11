//! The `rects` and `paddings` maps kept across updates and patched only
//! for the changed slots (IU3 3c-3b, `docs/plans/iu3.md`): a rebuilt or
//! moved element's entries are replaced, a freed one's removed, and an
//! update that changes no slot patches none. A debug build also
//! compares the kept maps with freshly built ones on every update that
//! paints (`kept_paint_check.rs`).

mod support;

use support::{document, element, viewport};
use vapors_dom::{Dom, NodeId};
use vapors_elements::Document;

/// Fixed-size blocks in a fixed-size body, so an edit moves only what
/// it names.
const CSS: &str = "body { margin: 0; height: 100px; } \
    div { box-sizing: border-box; height: 10px; } \
    .tall { height: 30px; } .bordered { border: 3px solid black; }";

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

    // Then: body and the first block rebuilt, the second freed
    assert_eq!(painted(&doc).border_box(second), None);
    assert_eq!(painted(&doc).padding_origin(second), None);
    assert_eq!(doc.update_counts().map_entries_patched, 3);
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
fn an_update_that_changes_no_slot_patches_no_entry() {
    // Given
    let (mut doc, _, _) = two_blocks();

    // When
    updated(&mut doc);

    // Then
    assert_eq!(doc.update_counts().map_entries_patched, 0);
}
