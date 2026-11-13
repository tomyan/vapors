//! A hoisted node inside a scroll container (review-v258 L1): its box
//! is placed in the container's frame but outside its parent node's
//! box, so its bounds are from the viewport, and a scroll of the
//! container moves it on the page while moving no box. The take after
//! the scroll reads it again, so its record follows; a hoisted node in
//! no scrolled frame is not read.

mod support;

use std::cell::RefCell;
use std::rc::Rc;

use support::mirror::step;
use support::mirror_model::Model;
use support::roles_page::button;
use support::{document, element, viewport};
use vapors_dom::{Dom, NodeId};
use vapors_elements::{AxMirrorNode, Document};

const STYLE: &str = "body { margin: 0; } \
    .scroller { overflow: auto; width: 200px; height: 100px; } \
    .relative { position: relative; height: 500px; } \
    .item { height: 20px; } \
    .abs { position: absolute; left: 10px; top: 30px; display: flex; } \
    .fixed { position: fixed; left: 300px; top: 300px; display: flex; }";

/// The review's shape, in a scroll container:
/// `div.relative > div[role=list] > div[role=listitem] > div.abs >
/// button`, as divs (vapors has no `ul` or `li` yet).
struct Page {
    doc: Rc<RefCell<Document>>,
    scroller: NodeId,
    button: NodeId,
}

fn page() -> Page {
    page_with(false)
}

/// [`page`], with a second hoisted button, `position: fixed` and so in
/// no scroll frame, in the list item when `fixed` is set.
fn page_with(fixed: bool) -> Page {
    let mut doc = document();
    let body = doc.body();
    let scroller = element(&mut doc, body, "div", "scroller");
    let relative = element(&mut doc, scroller, "div", "relative");
    let list = element(&mut doc, relative, "div", "");
    doc.set_attribute(list, "role", "list");
    let item = element(&mut doc, list, "div", "item");
    doc.set_attribute(item, "role", "listitem");
    let abs = element(&mut doc, item, "div", "abs");
    let button = button(&mut doc, abs, "Hoisted", &[]);
    if fixed {
        let pinned = element(&mut doc, item, "div", "fixed");
        support::roles_page::button(&mut doc, pinned, "Pinned", &[]);
    }
    doc.add_stylesheet("hoisted.css", STYLE).expect("parses");
    doc.update(viewport(640.0, 400.0)).expect("updates");
    Page {
        doc: Rc::new(RefCell::new(doc)),
        scroller,
        button,
    }
}

/// `page`, taken once, and the model built from that take.
fn taken(page: &Page) -> Model {
    let first = page.doc.borrow_mut().take_ax_update().expect("updated");
    let model = Model::from_every(&first.expect("every"));
    model.assert_mirrors(&page.doc.borrow());
    model
}

/// The record of `node` as a first take would hold it now.
fn record(doc: &Rc<RefCell<Document>>, node: NodeId) -> AxMirrorNode {
    let snapshot = doc.borrow().ax_mirror_snapshot().expect("snapshot");
    snapshot
        .into_iter()
        .find(|record| record.node == node)
        .expect("a record")
}

#[test]
fn a_hoisted_node_in_a_scrolled_container_follows_the_scroll() {
    // Given
    let page = page();
    let mut model = taken(&page);
    let before = record(&page.doc, page.button);
    assert!(before.hoisted, "{before:?}");
    assert_eq!(before.bounds.y, 30.0);

    // When
    let scrolled = page
        .doc
        .borrow_mut()
        .scroll_node_by(page.scroller, [0.0, 20.0]);
    assert!(scrolled);
    let update = step(&page.doc, &mut model).expect("the scroll");

    // Then: the model held against the snapshot (in `step`), and the
    // button's record moved up by the scroll
    let moved = update.nodes.iter().find(|node| node.node == page.button);
    assert_eq!(moved.map(|node| node.bounds.y), Some(10.0), "{update:?}");
}

#[test]
fn a_hoisted_node_follows_a_scroll_offset_an_update_clamps() {
    // Given: scrolled 300 of 400 px
    let page = page();
    let mut model = taken(&page);
    let scrolled = page
        .doc
        .borrow_mut()
        .scroll_node_by(page.scroller, [0.0, 300.0]);
    assert!(scrolled);
    step(&page.doc, &mut model).expect("the scroll");

    // When: the content shrinks to 200 px, so the offset is clamped to
    // 100 px, and the button's box does not move
    let relative = page.doc.borrow().children(page.scroller)[0];
    let shrink = "height: 200px";
    page.doc
        .borrow_mut()
        .set_attribute(relative, "style", shrink);
    step(&page.doc, &mut model).expect("the clamp");

    // Then: held against the snapshot (in `step`), 30 - 100 px
    assert_eq!(record(&page.doc, page.button).bounds.y, -70.0);
}

#[test]
fn a_scroll_reads_no_hoisted_node_outside_the_scrolled_frame() {
    // Given: the page with and without a fixed button in the list item
    let reads = |fixed: bool| {
        let page = page_with(fixed);
        let mut model = taken(&page);

        // When
        let mut doc = page.doc.borrow_mut();
        assert!(doc.scroll_node_by(page.scroller, [0.0, 20.0]));
        drop(doc);
        step(&page.doc, &mut model).expect("the scroll");
        page.doc.borrow().ax_take_counts().nodes_read
    };

    // Then: the fixed button is not read
    assert_eq!(reads(true), reads(false));
}
