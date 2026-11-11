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

/// Checks `doc`'s kept boxes against a fresh build.
fn check(doc: &Document) {
    let source = Source {
        arena: &doc.arena,
        used: &doc.kept.as_ref().expect("updated").used,
        images: &doc.images,
        text_marks: None,
    };
    check_against_fresh(&doc.boxes.placed, &source, doc.root());
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
