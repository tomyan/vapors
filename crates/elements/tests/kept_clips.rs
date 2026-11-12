//! Clips kept by slot across updates and worked out again only for the
//! boxes built again or moved and, below them, the boxes whose clip
//! comes from a box whose clip changed (IU3 3c-3c, `docs/plans/iu3.md`).
//! A debug build also compares the kept clips with fresh ones on every
//! update that paints (`kept_paint_check.rs`).

mod support;

use support::{document, element, viewport};
use vapors_dom::{Dom, NodeId};
use vapors_elements::Document;

/// `.column` lays its children out top to bottom and `.fill` takes what
/// is left of it, so growing `.after` shrinks `.fill` and keeps its
/// children where they are.
const CSS: &str = "body { margin: 0; height: 100px; } \
    div { box-sizing: border-box; } \
    .column { display: flex; flex-direction: column; height: 40px; } \
    .fill { flex: 1 1 0; min-height: 0; } \
    .clipper { overflow: hidden; } \
    .short { height: 10px; } .tall { height: 30px; } \
    .narrow { width: 10px; }";

fn updated(doc: &mut Document) {
    doc.update(viewport(200.0, 200.0)).expect("updates");
}

fn hit(doc: &Document, point: [f32; 2]) -> Option<NodeId> {
    doc.painted().expect("the last update painted").hit(point)
}

fn clips_redone(doc: &Document) -> usize {
    doc.update_counts().clips_redone
}

fn styled() -> (Document, NodeId) {
    let mut doc = document();
    doc.add_stylesheet("app.css", CSS).expect("parses");
    let body = doc.body();
    (doc, body)
}

#[test]
fn a_clipping_box_that_shrinks_clips_its_kept_child_again() {
    // Given: a 30px clipper over a narrow 10px block in a 40px column,
    // the clipper's child as tall as the clipper
    let (mut doc, body) = styled();
    let column = element(&mut doc, body, "div", "column");
    let clipper = element(&mut doc, column, "div", "fill clipper");
    let child = element(&mut doc, clipper, "div", "tall");
    let after = element(&mut doc, column, "div", "short narrow");
    updated(&mut doc);
    assert_eq!(hit(&doc, [50.0, 15.0]), Some(child));

    // When: the narrow block grows, the clipper shrinks to 10px
    doc.set_attribute(after, "class", "tall narrow");
    updated(&mut doc);

    // Then: the child is cut at 10px, though it did not move
    assert_eq!(hit(&doc, [50.0, 15.0]), Some(column));
    assert_eq!(clips_redone(&doc), 3, "the block, the clipper, its child");
}

#[test]
fn a_changed_clipper_below_an_unchanged_clip_is_clipped_again() {
    // Given: a 10px clipper holding a 30px child, in a wrapper that
    // does not clip, after a 10px block
    let (mut doc, body) = styled();
    let before = element(&mut doc, body, "div", "short");
    let wrapper = element(&mut doc, body, "div", "");
    let clipper = element(&mut doc, wrapper, "div", "short clipper");
    let child = element(&mut doc, clipper, "div", "tall");
    updated(&mut doc);
    assert_eq!(hit(&doc, [1.0, 15.0]), Some(child));

    // When: the block grows by 20px, moving the rest down; the
    // wrapper's clip, none, is unchanged
    doc.set_attribute(before, "class", "tall");
    updated(&mut doc);

    // Then: the clipper and its child are clipped where they now are
    assert_eq!(hit(&doc, [1.0, 35.0]), Some(child));
    assert_ne!(hit(&doc, [1.0, 45.0]), Some(child));
    assert_eq!(
        clips_redone(&doc),
        4,
        "the block, the wrapper, the clipper, its child"
    );
}

#[test]
fn overflow_toggled_to_hidden_clips_the_children_again() {
    // Given: a 10px block holding a 30px child
    let (mut doc, body) = styled();
    let block = element(&mut doc, body, "div", "short");
    let child = element(&mut doc, block, "div", "tall");
    updated(&mut doc);
    assert_eq!(hit(&doc, [1.0, 15.0]), Some(child));

    // When
    doc.set_attribute(block, "class", "short clipper");
    updated(&mut doc);

    // Then
    assert_ne!(hit(&doc, [1.0, 15.0]), Some(child));
    assert_eq!(doc.update_counts().order_walks, 0);

    // When: and back
    doc.set_attribute(block, "class", "short");
    updated(&mut doc);

    // Then
    assert_eq!(hit(&doc, [1.0, 15.0]), Some(child));
}
