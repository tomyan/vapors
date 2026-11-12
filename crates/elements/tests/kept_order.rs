//! Paint order, containing blocks and scroll chains kept across updates
//! and walked again over the whole tree only when its shape changes
//! (IU3 3c-3c, `docs/plans/iu3.md`): a box added or moved among its
//! siblings, or a rebuilt box's stacking changed. A debug build also
//! compares what is kept with what a fresh walk makes on every update
//! that paints (`kept_paint_check.rs`), so a missed shape change fails
//! there too. Clips: `kept_clips.rs`.

mod support;

use support::{document, element, viewport};
use vapors_dom::{Dom, NodeId};
use vapors_elements::Document;

/// Fixed-size blocks in a fixed-size body; `.over` pulls a block up
/// over the half of the one before it.
const CSS: &str = "body { margin: 0; height: 100px; } \
    div { box-sizing: border-box; height: 10px; } \
    .over { margin-top: -5px; } .relative { position: relative; } \
    .raised { position: relative; z-index: 1; } \
    .outlined { outline: 2px solid black; } \
    .scroller { height: 20px; overflow: auto; }";

/// A document of two blocks, the second over the first's lower half,
/// updated once.
fn overlapping_blocks() -> (Document, NodeId, NodeId) {
    let mut doc = document();
    doc.add_stylesheet("app.css", CSS).expect("parses");
    let body = doc.body();
    let first = element(&mut doc, body, "div", "relative");
    let second = element(&mut doc, body, "div", "over");
    updated(&mut doc);
    (doc, first, second)
}

fn updated(doc: &mut Document) {
    doc.update(viewport(200.0, 200.0)).expect("updates");
}

fn hit(doc: &Document, point: [f32; 2]) -> Option<NodeId> {
    doc.painted().expect("the last update painted").hit(point)
}

/// Order and chain walks of the last update.
fn walks(doc: &Document) -> (usize, usize) {
    let counts = doc.update_counts();
    (counts.order_walks, counts.chain_walks)
}

#[test]
fn an_edit_that_keeps_the_shape_walks_no_order_or_chain() {
    // Given
    let (mut doc, first, _) = overlapping_blocks();

    // When: a class that changes no stacking
    doc.set_attribute(first, "class", "relative outlined");
    updated(&mut doc);

    // Then
    assert_eq!(walks(&doc), (0, 0));
    assert!(doc.update_counts().boxes_painted > 0, "the update painted");
}

#[test]
fn an_outline_added_by_a_class_paints_with_the_kept_order() {
    // Given
    let (mut doc, _, second) = overlapping_blocks();

    // When: every box has an outline step, so none is added
    doc.set_attribute(second, "class", "over outlined");
    updated(&mut doc);

    // Then: the box and its outline drawn, order kept
    assert_eq!(walks(&doc), (0, 0));
    assert_eq!(doc.update_counts().scene_items_built, 1, "the outline");
}

#[test]
fn a_child_moved_before_its_sibling_walks_order_again() {
    // Given
    let (mut doc, first, second) = overlapping_blocks();
    assert_eq!(hit(&doc, [1.0, 7.0]), Some(first), "relative paints last");
    doc.set_attribute(first, "class", "");
    updated(&mut doc);
    assert_eq!(hit(&doc, [1.0, 7.0]), Some(second));

    // When: the same two keys, in the other order
    let body = doc.body();
    doc.insert_before(body, second, Some(first));
    updated(&mut doc);

    // Then
    assert_eq!(walks(&doc), (1, 1));
    assert_eq!(hit(&doc, [1.0, 1.0]), Some(second));
}

#[test]
fn a_z_index_set_on_a_rebuilt_box_walks_order_again() {
    // Given: the second paints over the first, later in tree order
    let (mut doc, first, second) = overlapping_blocks();
    doc.set_attribute(second, "class", "over relative");
    updated(&mut doc);
    assert_eq!(hit(&doc, [1.0, 7.0]), Some(second));

    // When: the first is raised; its children are unchanged
    doc.set_attribute(first, "class", "raised");
    updated(&mut doc);

    // Then
    assert_eq!(walks(&doc), (1, 1));
    assert_eq!(hit(&doc, [1.0, 7.0]), Some(first));
}

#[test]
fn position_static_to_relative_walks_order_again() {
    // Given: neither positioned, so the second paints over the first
    let (mut doc, first, second) = overlapping_blocks();
    doc.set_attribute(first, "class", "");
    updated(&mut doc);
    assert_eq!(hit(&doc, [1.0, 7.0]), Some(second));

    // When
    doc.set_attribute(first, "class", "relative");
    updated(&mut doc);

    // Then: positioned paints after in flow
    assert_eq!(walks(&doc), (1, 1));
    assert_eq!(hit(&doc, [1.0, 7.0]), Some(first));
}

#[test]
fn an_inserted_block_walks_order_and_chains_again() {
    // Given
    let (mut doc, _, _) = overlapping_blocks();

    // When
    let body = doc.body();
    let third = element(&mut doc, body, "div", "");
    updated(&mut doc);

    // Then
    assert_eq!(walks(&doc), (1, 1));
    assert_eq!(hit(&doc, [1.0, 16.0]), Some(third));
}

#[test]
fn overflow_toggled_to_scroll_walks_chains_again() {
    // Given: a block of three blocks
    let (mut doc, first, _) = overlapping_blocks();
    let inner: Vec<NodeId> = (0..3)
        .map(|_| element(&mut doc, first, "div", ""))
        .collect();
    updated(&mut doc);

    // When: the block scrolls
    doc.set_attribute(first, "class", "scroller");
    updated(&mut doc);

    // Then: a scroll over its child moves it
    assert_eq!(walks(&doc), (1, 1));
    assert!(doc.scroll_to(first, [0.0, 10.0]), "scrolls");
    updated(&mut doc);
    assert_eq!(hit(&doc, [1.0, 1.0]), Some(inner[1]));
}
