use taffy::Dimension;
use vapors_dom::{Dom, NodeId};
use vapors_text::FontSystem;

use super::check_against_fresh;
use crate::document::Document;
use crate::layout::Source;
use crate::layout::slots::Key;
use crate::update::Viewport;

const VIEWPORT: Viewport = Viewport {
    width: 200.0,
    height: 100.0,
    scale: 1.0,
};

/// Two divs under body, updated once; the second.
fn two_divs() -> (Document, NodeId) {
    let mut doc = Document::new(FontSystem::bundle_only());
    let body = doc.body();
    let divs: Vec<NodeId> = (0..2)
        .map(|_| {
            let node = doc.create_element("div");
            doc.append(body, node);
            node
        })
        .collect();
    doc.update(VIEWPORT).expect("updates");
    (doc, divs[1])
}

/// Checks `doc`'s kept boxes against a fresh build; none was built.
fn check(doc: &Document) {
    let source = Source {
        arena: &doc.arena,
        used: &doc.kept.as_ref().expect("updated").used,
        images: &doc.images,
        text_marks: None,
    };
    check_against_fresh(&doc.boxes.placed, &source, doc.root(), &[]);
}

#[test]
fn kept_boxes_a_fresh_build_makes_alike_pass() {
    // Given
    let (doc, _) = two_divs();

    // When, then: no panic
    check(&doc);
}

#[test]
#[should_panic(expected = "is not the box a fresh build makes")]
fn a_kept_box_whose_style_a_fresh_build_makes_differently_panics() {
    // Given: the second div's kept box has a height its style lacks
    let (mut doc, second) = two_divs();
    let slot = doc.boxes.placed.slots.of(Key::Element(second));
    let slot = slot.expect("holds a slot");
    doc.boxes.placed.boxes[slot].style.size.height = Dimension::length(5.0);

    // When, then
    check(&doc);
}

/// The second div's kept box, in `doc`.
fn second_box(doc: &mut Document, second: NodeId) -> &mut crate::layout::LayoutBox {
    let slot = doc.boxes.placed.slots.of(Key::Element(second));
    &mut doc.boxes.placed.boxes[slot.expect("holds a slot")]
}

#[test]
#[should_panic(expected = "is not the box a fresh build makes")]
fn a_kept_box_with_paint_a_fresh_build_makes_differently_panics() {
    // Given: the second div's kept box says it is a checked checkbox
    let (mut doc, second) = two_divs();
    second_box(&mut doc, second).checked = Some(true);

    // When, then
    check(&doc);
}

#[test]
#[should_panic(expected = "is not the box a fresh build makes")]
fn a_box_not_built_this_update_that_is_not_flagged_kept_panics() {
    // Given
    let (mut doc, second) = two_divs();
    second_box(&mut doc, second).kept = false;

    // When, then
    check(&doc);
}

#[test]
#[should_panic(expected = "is not the box a fresh build makes")]
fn a_slot_whose_key_is_not_its_boxs_panics() {
    // Given: the second div's slot holds the first text key of body
    let (mut doc, second) = two_divs();
    let slot = doc.boxes.placed.slots.of(Key::Element(second));
    let body = doc.body();
    doc.boxes.placed.slots.keys[slot.expect("holds a slot")] = Some(Key::Text(body, 0));

    // When, then
    check(&doc);
}
