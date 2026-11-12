//! Bounds relative to the node above, and scroll containers as nodes
//! (R6 PR 5a, `docs/plans/r6.md` "Element mapping"): a record's bounds
//! are its border box from the padding box of its parent node, plus
//! the parent's scroll offset, so a scroll changes no row's record,
//! only the scroller's offset; the mirror writes one scroll. A scroll
//! container with no role is a node of role `none` with its offset
//! and extent, so the mirror can nest its rows under a scrolling
//! element.

// row heights of 24 and a content height of 72,000: exact in f32
#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]

mod support;

use std::cell::RefCell;
use std::rc::Rc;

use support::mirror::{node_of, step, taken_model};
use support::mirror_model::Model;
use support::rows_page::{ROW_HEIGHT, RowsPage, rows_page};
use vapors_dom::{Dom, NodeId};
use vapors_elements::{AxMirrorNode, Document};

const ROWS: usize = 3000;

/// `ROWS` rows in a scrolling body, taken once: the page and its model.
fn scrolling_rows() -> (RowsPage, Model) {
    let page = rows_page(ROWS);
    page.body_scrolls();
    let first = page
        .doc
        .borrow_mut()
        .take_ax_update()
        .expect("updated")
        .expect("every");
    let model = Model::from_every(&first);
    model.assert_mirrors(&page.doc.borrow());
    (page, model)
}

/// The record of `node` as a first take would hold it now.
fn record(doc: &Rc<RefCell<Document>>, node: NodeId) -> AxMirrorNode {
    let doc = doc.borrow();
    doc.ax_mirror_snapshot()
        .expect("snapshot")
        .into_iter()
        .find(|record| record.node == node)
        .unwrap_or_else(|| panic!("no record for {node:?}"))
}

#[test]
fn a_scrolling_body_is_a_node_of_role_none_holding_its_rows() {
    // Given / When
    let (page, _) = scrolling_rows();
    let body = page.doc.borrow().body();
    let body_record = record(&page.doc, body);
    let button = record(&page.doc, page.rows[1].button);

    // Then
    assert_eq!(
        (body_record.role.as_str(), body_record.parent),
        ("none", None)
    );
    assert_eq!(button.parent, Some(body));
    let scroll = body_record.scroll.expect("a scroll container");
    assert_eq!(scroll.offset, [0.0, 0.0]);
    let content = (ROWS as u32 * ROW_HEIGHT) as f32;
    assert_eq!(scroll.extent[1], content, "{scroll:?}");
    assert_eq!(scroll.extent[0], body_record.bounds.width, "{scroll:?}");
}

#[test]
fn a_rows_bounds_are_from_its_scroller_and_do_not_change_when_it_scrolls() {
    // Given
    let (page, mut model) = scrolling_rows();
    let button = page.rows[1].button;
    let before = record(&page.doc, button).bounds;
    assert_eq!(before.y, ROW_HEIGHT as f32, "{before:?}");

    // When
    assert!(page.scroll_body(1.0));
    step(&page.doc, &mut model);

    // Then: the same place in the body's content
    assert_eq!(record(&page.doc, button).bounds, before);
}

#[test]
fn a_scroll_emits_the_scrollers_offset_and_reads_no_row() {
    // Given
    let (page, mut model) = scrolling_rows();
    let body = page.doc.borrow().body();

    // When
    assert!(page.scroll_body(1.0));
    let update = step(&page.doc, &mut model).expect("the scroll");

    // Then: one record, the scroller's, with its new offset
    assert_eq!(update.nodes.len(), 1, "{update:?}");
    assert_eq!(update.nodes[0].node, body);
    let scroll = update.nodes[0].scroll.expect("a scroll container");
    assert_eq!(scroll.offset, [0.0, ROW_HEIGHT as f32]);
    assert!(update.placed.is_empty() && update.removed.is_empty());
    let counts = page.doc.borrow().ax_take_counts();
    assert!(counts.nodes_read <= 2, "{counts:?}");
}

#[test]
fn a_child_nodes_bounds_are_from_its_parent_nodes_border_box() {
    // Given: the roles page, where a button sits in a toolbar
    let (doc, _) = taken_model();
    let pin = node_of(&doc, "button", "Pin");
    let pin_record = record(&doc, pin);
    let parent = pin_record.parent.expect("under a node");

    // When
    let (pin_box, parent_box) = {
        let doc = doc.borrow();
        let painted = doc.painted().expect("painted");
        (
            painted.border_box(pin).expect("a box"),
            painted.border_box(parent).expect("a box"),
        )
    };

    // Then: the mirror's parent element has no border, so its padding
    // box is the record's border box
    assert_eq!(pin_record.bounds.x, pin_box.x - parent_box.x);
    assert_eq!(pin_record.bounds.y, pin_box.y - parent_box.y);
    assert_eq!(pin_record.bounds.width, pin_box.width);
}

#[test]
fn an_update_that_clamps_a_scrollers_offset_emits_the_new_offset() {
    // Given: scrolled to the bottom, then half the rows removed
    let (page, mut model) = scrolling_rows();
    let body = page.doc.borrow().body();
    assert!(page.scroll_body(3000.0));
    step(&page.doc, &mut model).expect("the scroll");
    let at_bottom = record(&page.doc, body).scroll.expect("scrolls").offset;
    {
        let mut doc = page.doc.borrow_mut();
        for row in &page.rows[ROWS / 2..] {
            doc.remove(row.element);
        }
    }

    // When
    let update = step(&page.doc, &mut model).expect("the removal");

    // Then: the body's record carries the clamped offset
    let body_record = update
        .nodes
        .iter()
        .find(|node| node.node == body)
        .expect("the body read again");
    let offset = body_record.scroll.expect("scrolls").offset;
    assert!(offset[1] < at_bottom[1], "{offset:?} < {at_bottom:?}");
    assert_eq!(
        offset,
        record(&page.doc, body).scroll.expect("scrolls").offset
    );
}
