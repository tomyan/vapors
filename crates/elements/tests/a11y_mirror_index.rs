//! The index of ids a take reads names through is patched, not rebuilt,
//! for a take that changes elements in place (R6, review 230-M3): a
//! scroll, a focus move or a state change on a page whose controls
//! carry ids costs the nodes touched, never a scan of the page. A take
//! that inserts or removes elements scans once, as does one that finds
//! an id carried by more than one element, since only document order
//! says which element an id names.

mod support;

use std::cell::RefCell;
use std::rc::Rc;

use support::mirror::{labels, node_of, step, taken_model};
use support::{element, text};
use vapors_dom::{Dom, NodeId};
use vapors_elements::Document;

fn scans(doc: &Rc<RefCell<Document>>) -> usize {
    doc.borrow().ax_index_scans()
}

#[test]
fn the_first_take_scans_once() {
    // Given, When
    let (doc, _model) = taken_model();

    // Then
    assert_eq!(scans(&doc), 1);
}

#[test]
fn a_scroll_a_focus_move_and_a_state_change_scan_nothing() {
    // Given: the region scrolls; the page's inputs carry ids
    let (doc, mut model) = taken_model();
    let region = node_of(&doc, "region", "Options");
    let pin = node_of(&doc, "button", "Pin");
    {
        let mut doc = doc.borrow_mut();
        doc.add_stylesheet(
            "scroll.css",
            ".sc { overflow: auto; width: 200px; } .sc > * { flex-shrink: 0; }",
        )
        .expect("parses");
        doc.set_attribute(region, "class", "row sc");
    }
    step(&doc, &mut model);
    let before = scans(&doc);

    // When
    assert!(
        doc.borrow_mut().scroll_node_by(region, [60.0, 0.0]),
        "scrolls"
    );
    step(&doc, &mut model).expect("bounds moved");
    Document::focus(&doc, Some(pin));
    step(&doc, &mut model).expect("focus moved");
    doc.borrow_mut().set_attribute(pin, "aria-pressed", "true");
    step(&doc, &mut model).expect("a state");

    // Then
    assert_eq!(scans(&doc), before);
}

#[test]
fn an_insert_and_a_removal_each_scan_once() {
    // Given
    let (doc, mut model) = taken_model();
    let region = node_of(&doc, "region", "Options");
    let before = scans(&doc);
    let added = {
        let mut doc = doc.borrow_mut();
        let added = element(&mut doc, region, "button", "");
        text(&mut doc, added, "Undo");
        added
    };

    // When
    step(&doc, &mut model).expect("the insert");
    let after_insert = scans(&doc);
    doc.borrow_mut().remove(added);
    step(&doc, &mut model).expect("the removal");

    // Then
    assert_eq!((after_insert, scans(&doc)), (before + 1, before + 2));
}

/// A span `#lab` with `content` appended to `row`.
fn label(doc: &mut Document, row: NodeId, content: &str) -> NodeId {
    let label = element(doc, row, "span", "");
    doc.set_attribute(label, "id", "lab");
    text(doc, label, content);
    label
}

#[test]
fn an_id_carried_twice_names_by_document_order_after_the_first_gives_it_up() {
    // Given: two spans #lab in After's row, After labelled by "lab"
    let (doc, mut model) = taken_model();
    let after = node_of(&doc, "button", "After");
    let row = doc.borrow().parent(after).expect("the row");
    let first = {
        let mut doc = doc.borrow_mut();
        let first = label(&mut doc, row, "First");
        label(&mut doc, row, "Second");
        doc.set_attribute(after, "aria-labelledby", "lab");
        first
    };
    let named = step(&doc, &mut model).expect("the labels");
    assert_eq!(labels(&named.nodes), ["button \"First\""]);
    let before = scans(&doc);
    doc.borrow_mut().set_attribute(first, "id", "was-lab");

    // When: the first gives up the id in place, which only a scan settles
    let update = step(&doc, &mut model).expect("a rename");

    // Then
    assert_eq!(labels(&update.nodes), ["button \"Second\""]);
    assert_eq!(scans(&doc), before + 1);
}
