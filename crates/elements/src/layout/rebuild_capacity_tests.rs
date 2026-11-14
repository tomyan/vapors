//! The boxes a full build keeps hold no spare slots, vacant ones
//! included (property 8: grown by doubling, 40,002 boxes of about
//! 2.8 KB each held 65,536 slots).

use vapors_dom::{Dom, NodeId};
use vapors_text::FontSystem;

use crate::document::Document;
use crate::update::Viewport;

fn viewport(scale: f32) -> Viewport {
    Viewport {
        width: 200.0,
        height: 100.0,
        scale,
    }
}

/// html, body and `count` divs, updated once; the divs.
fn divs(count: usize) -> (Document, Vec<NodeId>) {
    let mut doc = Document::new(FontSystem::bundle_only());
    let body = doc.body();
    let made = (0..count)
        .map(|_| {
            let div = doc.create_element("div");
            doc.append(body, div);
            div
        })
        .collect();
    doc.update(viewport(1.0)).expect("updates");
    (doc, made)
}

fn slots(doc: &Document) -> (usize, usize) {
    let boxes = &doc.boxes.placed.boxes;
    (boxes.len(), boxes.capacity())
}

#[test]
fn a_full_build_keeps_its_boxes_in_as_many_slots_as_boxes() {
    // When: html, body and five divs, seven boxes, grown to eight slots
    let (doc, _) = divs(5);

    // Then
    assert_eq!(slots(&doc), (7, 7));
}

#[test]
fn a_full_build_with_a_vacant_slot_keeps_no_spare_slots() {
    // Given: a div removed, its slot left vacant
    let (mut doc, made) = divs(6);
    doc.remove(made[2]);
    doc.update(viewport(1.0)).expect("updates");

    // When: a scale change builds every box again, the vacant slot kept
    doc.update(viewport(2.0)).expect("updates");

    // Then
    let (len, capacity) = slots(&doc);
    assert_eq!(capacity, len, "{len} slots in {capacity}");
}

#[test]
fn a_box_added_after_a_full_build_grows_the_list_by_a_quarter() {
    // Given: 40 divs, 42 boxes in 42 slots
    let (mut doc, made) = divs(40);
    assert_eq!(slots(&doc), (42, 42));

    // When: one div more, built into a new slot
    let body = doc.body();
    let div = doc.create_element("div");
    doc.insert_before(body, div, Some(made[20]));
    doc.update(viewport(1.0)).expect("updates");

    // Then: 42 + 42 / 4 slots, not 84, for the boxes and their keys
    assert_eq!(slots(&doc), (43, 52));
    let keys = &doc.boxes.placed.slots.keys;
    assert_eq!((keys.len(), keys.capacity()), (43, 52));
}
