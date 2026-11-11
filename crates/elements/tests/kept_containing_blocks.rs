//! Containing blocks, clips and scroll chains walked again when a
//! rebuilt box changes what decides containing blocks (IU3 3c-3c,
//! review of #198): whether it is the containing block of absolute or
//! fixed boxes, and whether it is out of flow. Its children and
//! stacking can stay equal while either changes. A debug build also
//! compares the kept clip tree, clips and chains with fresh ones
//! (`kept_paint_check.rs`), so a missed change panics there too.

mod support;

use support::{document, element, viewport};
use vapors_dom::{Dom, NodeId};
use vapors_elements::Document;

/// A relative clipper 100px tall around a 10px clipper, so a box
/// placed against the outer one shows below 10px and one in flow under
/// the inner one does not.
const CSS: &str = "body { margin: 0; } div { box-sizing: border-box; } \
    .outer { position: relative; overflow: hidden; height: 100px; } \
    .mid { overflow: hidden; height: 10px; } \
    .scroller { overflow: auto; height: 10px; } \
    .abs { position: absolute; top: 0; left: 0; width: 50px; height: 50px; } \
    .rel { position: relative; width: 50px; height: 50px; } \
    .flex { display: flex; } .z { z-index: 1; width: 100px; } \
    .pos { position: relative; }";

/// A point below the inner clipper and inside the outer one.
const BELOW_INNER: [f32; 2] = [5.0, 30.0];

/// `.outer` around a box of class `inner` around a box of class `leaf`,
/// updated once: the document, the inner box and the leaf.
fn nested(outer: &str, inner: &str, leaf: &str) -> (Document, NodeId, NodeId) {
    let mut doc = document();
    doc.add_stylesheet("app.css", CSS).expect("parses");
    let body = doc.body();
    let outer = element(&mut doc, body, "div", outer);
    let inner = element(&mut doc, outer, "div", inner);
    let leaf = element(&mut doc, inner, "div", leaf);
    updated(&mut doc);
    (doc, inner, leaf)
}

fn updated(doc: &mut Document) {
    doc.update(viewport(200.0, 200.0)).expect("updates");
}

fn hit(doc: &Document, point: [f32; 2]) -> Option<NodeId> {
    doc.painted().expect("painted").hit(point)
}

#[test]
fn an_absolute_box_made_relative_takes_its_parents_clip() {
    // Given: an absolute box placed against the outer clipper
    let (mut doc, _, leaf) = nested("outer", "mid", "abs");
    assert_eq!(hit(&doc, BELOW_INNER), Some(leaf));

    // When: it is made relative, in flow under the inner clipper
    doc.set_attribute(leaf, "class", "rel");
    updated(&mut doc);

    // Then: the inner clipper clips it
    assert_ne!(hit(&doc, BELOW_INNER), Some(leaf));
}

#[test]
fn a_relative_box_made_absolute_takes_its_containing_blocks_clip() {
    // Given: a relative box in flow under the inner clipper
    let (mut doc, _, leaf) = nested("outer", "mid", "rel");
    assert_ne!(hit(&doc, BELOW_INNER), Some(leaf));

    // When: it is made absolute, placed against the outer clipper
    doc.set_attribute(leaf, "class", "abs");
    updated(&mut doc);

    // Then: only the outer clipper clips it
    assert_eq!(hit(&doc, BELOW_INNER), Some(leaf));
}

#[test]
fn an_absolute_box_made_relative_under_a_scroller_joins_its_chain() {
    // Given: an absolute box under a scroll container, placed against
    // the outer clipper
    let (mut doc, scroller, leaf) = nested("outer", "scroller", "abs");
    assert_eq!(hit(&doc, BELOW_INNER), Some(leaf));

    // When: it is made relative, in flow inside the scroll container
    doc.set_attribute(leaf, "class", "rel");
    updated(&mut doc);

    // Then: the scroll container clips it, and a scroll down over it
    // moves the scroll container
    assert_ne!(hit(&doc, BELOW_INNER), Some(leaf));
    assert_eq!(doc.scroll_target([5.0, 5.0], [0.0, 10.0]), Some(scroller));
}

#[test]
fn a_z_indexed_flex_item_made_relative_becomes_the_containing_block() {
    // Given: a flex item with a z-index, which stacks as positioned but
    // is not a containing block, around an absolute box
    let (mut doc, item, leaf) = nested("outer flex", "mid z", "abs");
    assert_eq!(hit(&doc, BELOW_INNER), Some(leaf));

    // When: the item is made relative
    doc.set_attribute(item, "class", "mid z pos");
    updated(&mut doc);

    // Then: the absolute box is placed against it and takes its clip
    assert_ne!(hit(&doc, BELOW_INNER), Some(leaf));
}
