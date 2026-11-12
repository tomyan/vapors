//! A hoisted node (R6 PR 5a, review-v243 H1 and M1): one whose box is
//! placed outside its parent node's box, an absolutely positioned
//! popover or toast whose containing block is above the parent. It does
//! not move with the parent, so its bounds are from the viewport and
//! its mirror element is `position: fixed`: right when the parent moves
//! without it, right while the parent's container is scrolled, never
//! silently off. A box made or unmade a containing block reads every
//! node again, so a node's hoisting follows its containing block.

// a row height of 24: exact in f32
#![allow(clippy::cast_precision_loss)]

mod support;

use std::cell::RefCell;
use std::rc::Rc;

use support::element;
use support::mirror::{node_of, step, taken_model};
use support::roles_page::button;
use support::rows_page::{ROW_HEIGHT, rows_page};
use vapors_dom::{Dom, NodeId};
use vapors_elements::{AxMirrorNode, Document};

/// The record of `node` as a first take would hold it now.
fn record(doc: &Rc<RefCell<Document>>, node: NodeId) -> AxMirrorNode {
    doc.borrow()
        .ax_mirror_snapshot()
        .expect("snapshot")
        .into_iter()
        .find(|record| record.node == node)
        .unwrap_or_else(|| panic!("no record for {node:?}"))
}

/// Button "Floating" in an absolutely positioned div under the region,
/// placed against the viewport, with the take after it applied.
fn floating(doc: &Rc<RefCell<Document>>) -> NodeId {
    let region = node_of(doc, "region", "Options");
    let mut doc = doc.borrow_mut();
    doc.add_stylesheet(
        "abs.css",
        ".abs { position: absolute; display: flex; left: 300px; top: 200px; } \
         .spacer { height: 50px; }",
    )
    .expect("parses");
    let wrap = element(&mut doc, region, "div", "abs");
    button(&mut doc, wrap, "Floating", &[]);
    wrap
}

/// A 50px spacer before the region, which moves the region down.
fn spacer_before_region(doc: &Rc<RefCell<Document>>) {
    let region = node_of(doc, "region", "Options");
    let mut doc = doc.borrow_mut();
    let dialog = doc.parent(region).expect("in the dialog");
    let spacer = doc.create_element("div");
    doc.set_attribute(spacer, "class", "spacer");
    doc.insert_before(dialog, spacer, Some(region));
}

#[test]
fn a_hoisted_node_keeps_its_viewport_bounds_when_its_parent_node_moves() {
    // Given: Floating under the region, placed against the viewport
    let (doc, mut model) = taken_model();
    floating(&doc);
    step(&doc, &mut model).expect("Floating's record");
    let floating = node_of(&doc, "button", "Floating");
    let before = record(&doc, floating);
    assert!(before.hoisted, "{before:?}");
    assert_eq!((before.bounds.x, before.bounds.y), (300.0, 200.0));

    // When: the region moves down and Floating does not (review-v243 H1's probe)
    spacer_before_region(&doc);
    let update = step(&doc, &mut model).expect("the region's move");

    // Then: the model held against the snapshot (in `step`), and
    // Floating's record is not among the changes
    assert!(
        update.nodes.iter().all(|node| node.node != floating),
        "{update:?}"
    );
    assert_eq!(record(&doc, floating), before);
}

#[test]
fn a_node_placed_in_its_parent_follows_it_and_is_not_hoisted() {
    // Given: the region made the containing block before Floating comes
    let (doc, mut model) = taken_model();
    let region = node_of(&doc, "region", "Options");
    floating(&doc);
    doc.borrow_mut()
        .set_attribute(region, "style", "position: relative");
    step(&doc, &mut model).expect("Floating's record");
    let floating = node_of(&doc, "button", "Floating");
    let before = record(&doc, floating);
    assert!(!before.hoisted, "{before:?}");

    // When: the region moves down, and Floating with it
    spacer_before_region(&doc);
    step(&doc, &mut model).expect("the region's move");

    // Then: the same place in the region
    assert_eq!(record(&doc, floating).bounds, before.bounds);
}

#[test]
fn a_node_whose_parent_stops_being_its_containing_block_is_hoisted_before_the_parent_moves() {
    // Given: Floating placed in the region (relative), then the region
    // unmade a containing block with no box moved
    let (doc, mut model) = taken_model();
    let region = node_of(&doc, "region", "Options");
    floating(&doc);
    doc.borrow_mut()
        .set_attribute(region, "style", "position: relative");
    step(&doc, &mut model).expect("Floating's record");
    let floating = node_of(&doc, "button", "Floating");
    doc.borrow_mut().set_attribute(region, "style", "");
    let update = step(&doc, &mut model).expect("every node read again");
    assert!(update.every, "{update:?}");
    assert!(record(&doc, floating).hoisted);

    // When: the region moves down without Floating
    spacer_before_region(&doc);
    step(&doc, &mut model).expect("the region's move");

    // Then: held against the snapshot in `step`; Floating where it was
    assert_eq!(
        (
            record(&doc, floating).bounds.x,
            record(&doc, floating).bounds.y
        ),
        (300.0, 200.0)
    );
}

#[test]
fn a_hoisted_node_in_a_scrolled_container_keeps_its_viewport_bounds_through_a_scroll() {
    // Given: a popover hoisted out of the scrolling body (review-v243 M1)
    let page = rows_page(200);
    page.body_scrolls();
    let (popover, button) = {
        let mut doc = page.doc.borrow_mut();
        doc.add_stylesheet(
            "pop.css",
            "html { position: relative; } .pop { position: absolute; left: 100px; top: 100px; display: flex; }",
        )
        .expect("parses");
        let body = doc.body();
        let pop = element(&mut doc, body, "div", "pop");
        let button = button(&mut doc, pop, "Popover", &[]);
        doc.update(support::rows_page::rows_viewport())
            .expect("updates");
        (pop, button)
    };
    let first = page
        .doc
        .borrow_mut()
        .take_ax_update()
        .expect("updated")
        .expect("every");
    let mut model = support::mirror_model::Model::from_every(&first);
    model.assert_mirrors(&page.doc.borrow());
    let before = record(&page.doc, button);
    assert!(before.hoisted, "{before:?} under {popover:?}");
    assert_eq!((before.bounds.x, before.bounds.y), (100.0, 100.0));

    // When
    assert!(page.scroll_body(2.0));
    let update = step(&page.doc, &mut model).expect("the scroll");

    // Then: the body's offset alone; the popover where it is on screen
    assert_eq!(update.nodes.len(), 1, "{update:?}");
    assert_eq!(
        update.nodes[0].scroll.expect("scrolls").offset[1],
        2.0 * ROW_HEIGHT as f32
    );
    assert_eq!(record(&page.doc, button), before);
}
