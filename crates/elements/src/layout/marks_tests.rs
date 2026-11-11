use vapors_dom::{Dom, NodeId};
use vapors_text::FontSystem;

use super::Rebuild;
use crate::document::Document;
use crate::layout::slots::Key;
use crate::update::Viewport;

const VIEWPORT: Viewport = Viewport {
    width: 200.0,
    height: 100.0,
    scale: 1.0,
};

/// An empty div appended to `parent`.
fn div(doc: &mut Document, parent: NodeId) -> NodeId {
    let node = doc.create_element("div");
    doc.append(parent, node);
    node
}

/// The slots of the divs appended into four columns after the first
/// update, one per column, so the update rebuilds four owners.
fn slots_of_new_divs() -> Vec<Option<usize>> {
    let mut doc = Document::new(FontSystem::bundle_only());
    let body = doc.body();
    let columns: Vec<NodeId> = (0..4).map(|_| div(&mut doc, body)).collect();
    doc.update(VIEWPORT).expect("updates");
    let added: Vec<NodeId> = columns
        .iter()
        .map(|&column| div(&mut doc, column))
        .collect();
    doc.update(VIEWPORT).expect("updates");
    let slots = &doc.boxes.placed.slots;
    added
        .iter()
        .map(|&node| slots.of(Key::Element(node)))
        .collect()
}

#[test]
fn new_keys_take_the_same_slots_on_every_run() {
    // Given
    let first = slots_of_new_divs();

    // When
    let runs: Vec<Vec<Option<usize>>> = (0..20).map(|_| slots_of_new_divs()).collect();

    // Then
    assert!(
        first.iter().all(Option::is_some),
        "each new div has a slot: {first:?}"
    );
    for run in runs {
        assert_eq!(run, first, "owners rebuilt in a different order");
    }
}

#[test]
fn a_mark_on_a_root_that_lost_its_box_rebuilds_every_box() {
    // Given: html with a box, then display: none
    let mut doc = Document::new(FontSystem::bundle_only());
    let body = doc.body();
    div(&mut doc, body);
    doc.add_stylesheet("app.css", ".hidden { display: none; }")
        .expect("parses");
    doc.update(VIEWPORT).expect("updates");
    let root = doc.root();
    doc.set_attribute(root, "class", "hidden");

    // When
    doc.restyle(VIEWPORT).expect("restyles");
    let used = &doc.kept.as_ref().expect("restyled").used;
    let rebuild = doc
        .boxes
        .marks
        .rebuild(&doc.arena, used, &doc.boxes.placed.slots, root);

    // Then
    assert_eq!(rebuild, Rebuild::Every);
}
