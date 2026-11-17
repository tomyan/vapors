use vapors_dom::{Dom, NodeId};
use vapors_text::FontSystem;

use super::{frame_chains, to_measure};
use crate::document::Document;
use crate::kept_order::KeptOrder;
use crate::painted::Painted;
use crate::update::Viewport;

const VIEWPORT: Viewport = Viewport {
    width: 640.0,
    height: 400.0,
    scale: 1.0,
};

/// Three scroll containers, each inside the last, and a leaf in the
/// innermost, updated once: its paint, with its order taken out.
pub(crate) fn nested_three() -> (Painted, KeptOrder) {
    let mut doc = Document::new(FontSystem::bundle_only());
    let css = ".scroller { overflow: auto; height: 50px; } .leaf { height: 100px; }";
    doc.add_stylesheet("t.css", css).expect("parses");
    let mut parent = doc.body();
    for _ in 0..3 {
        parent = child(&mut doc, parent, "scroller");
    }
    child(&mut doc, parent, "leaf");
    doc.update(VIEWPORT).expect("updates");
    let mut painted = doc.painted.take().expect("painted");
    let order = std::mem::take(&mut painted.order);
    (painted, order)
}

fn child(doc: &mut Document, parent: NodeId, class: &str) -> NodeId {
    let node = doc.create_element("div");
    doc.set_attribute(node, "class", class);
    doc.append(parent, node);
    node
}

fn painted_slots(painted: &Painted) -> Vec<usize> {
    let slots = 0..painted.kept_boxes.len();
    slots
        .filter(|&slot| painted.kept_boxes[slot].is_some())
        .collect()
}

#[test]
fn each_scroll_frame_has_a_link_for_its_own_container_and_each_around_it() {
    // Given
    let (painted, order) = nested_three();

    // When
    let chains = frame_chains(&order, &painted.frames);

    // Then: in slot order, outermost first
    let links: Vec<(u8, usize)> = chains
        .iter()
        .map(|&(kind, _, links)| (kind, links))
        .collect();
    assert_eq!(links, vec![(0, 1), (0, 2), (0, 3)]);
}

#[test]
fn frame_links_unchanged_since_the_last_measure_measure_only_the_boxes_painted_again() {
    // Given: the paint as measured at its update
    let (mut painted, order) = nested_three();

    // When
    let measured = to_measure(&order, &mut painted, &[3]);

    // Then
    assert_eq!(measured, vec![3]);
}

#[test]
fn a_frame_whose_links_changed_since_the_last_measure_measures_every_painted_box() {
    // Given: the paint as if last measured when its outermost scroller
    // had one link fewer
    let (mut painted, order) = nested_three();
    let current = painted.frame_chains.clone();
    painted.frame_chains[0].2 -= 1;

    // When
    let measured = to_measure(&order, &mut painted, &[3]);

    // Then: every box, and the links kept are the current ones
    assert_eq!(measured, painted_slots(&painted));
    assert_eq!(painted.frame_chains, current);
}
