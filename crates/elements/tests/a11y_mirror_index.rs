//! The index of ids a take reads names through is patched, not rebuilt,
//! for a take that changes elements in place (R6, review 230-M3): a
//! scroll, a focus move or a state change on a page whose controls
//! carry ids costs the nodes touched, never a scan of the page. A take
//! that inserts or removes elements patches the entries of the elements
//! in each inserted or removed subtree, hidden ones included (4i). A take
//! that finds an id carried by more than one element scans once, since
//! only document order says which element an id names. The counts are `AxTakeCounts`;
//! `a11y_mirror_scaling.rs` holds them across page sizes.

mod support;

use std::cell::RefCell;
use std::rc::Rc;

use support::mirror::{labels, node_of, step, taken_model};
use support::{element, text};
use vapors_dom::{Dom, NodeId};
use vapors_elements::Document;

/// Whether the last take scanned the document for its index.
fn scans(doc: &Rc<RefCell<Document>>) -> usize {
    doc.borrow().ax_take_counts().index_scans
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

    // When, Then
    assert!(
        doc.borrow_mut().scroll_node_by(region, [60.0, 0.0]),
        "scrolls"
    );
    step(&doc, &mut model).expect("bounds moved");
    assert_eq!(scans(&doc), 0, "a scroll");
    Document::focus(&doc, Some(pin));
    step(&doc, &mut model).expect("focus moved");
    assert_eq!(scans(&doc), 0, "a focus move");
    doc.borrow_mut().set_attribute(pin, "aria-pressed", "true");
    step(&doc, &mut model).expect("a state");
    assert_eq!(scans(&doc), 0, "a state change");
}

#[test]
fn an_insert_and_a_removal_scan_nothing() {
    // Given
    let (doc, mut model) = taken_model();
    let region = node_of(&doc, "region", "Options");
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
    assert_eq!((after_insert, scans(&doc)), (0, 0));
}

#[test]
fn an_element_moved_ahead_of_another_with_its_id_takes_the_name_by_a_scan() {
    // Given: two spans #lab in After's row, After labelled by the first
    let (doc, mut model) = taken_model();
    let after = node_of(&doc, "button", "After");
    let row = doc.borrow().parent(after).expect("the row");
    let (first, second) = {
        let mut doc = doc.borrow_mut();
        let first = label(&mut doc, row, "First");
        let second = label(&mut doc, row, "Second");
        doc.set_attribute(after, "aria-labelledby", "lab");
        (first, second)
    };
    step(&doc, &mut model).expect("the labels");

    // When: the second is moved ahead of the first
    doc.borrow_mut().insert_before(row, second, Some(first));
    let update = step(&doc, &mut model).expect("a rename");

    // Then: document order names the second now, which a scan settles
    assert_eq!(labels(&update.nodes), ["button \"Second\""]);
    assert_eq!(scans(&doc), 1);
}

/// A div holding a hidden span `#late` with `content`, put in `row`: the
/// span has no box, so no box touches it.
fn hidden_label(doc: &mut Document, row: NodeId, content: &str) -> NodeId {
    let holder = element(doc, row, "div", "");
    let late = element(doc, holder, "span", "");
    doc.set_attribute(late, "id", "late");
    doc.set_attribute(late, "hidden", "");
    text(doc, late, content);
    holder
}

#[test]
fn a_hidden_element_inserted_with_an_id_names_its_referrer_without_a_scan() {
    // Given: After labelled by "late", which no element carries yet
    let (doc, mut model) = taken_model();
    let after = node_of(&doc, "button", "After");
    let row = doc.borrow().parent(after).expect("the row");
    doc.borrow_mut()
        .set_attribute(after, "aria-labelledby", "late");
    step(&doc, &mut model);

    // When: a subtree holding a hidden #late is inserted
    hidden_label(&mut doc.borrow_mut(), row, "Later");
    let update = step(&doc, &mut model).expect("the insert");

    // Then: After is named by it, and the index was patched
    assert!(
        labels(&update.nodes).contains(&"button \"Later\"".to_owned()),
        "{:?}",
        labels(&update.nodes)
    );
    assert_eq!(scans(&doc), 0);
}

#[test]
fn a_subtree_built_apart_across_a_take_names_its_referrer_once_inserted() {
    // Given: After labelled by "late"; a subtree holding a hidden #late
    // built apart from the document, and a take made meanwhile
    let (doc, mut model) = taken_model();
    let after = node_of(&doc, "button", "After");
    let row = doc.borrow().parent(after).expect("the row");
    let holder = {
        let mut doc = doc.borrow_mut();
        doc.set_attribute(after, "aria-labelledby", "late");
        let holder = doc.create_element("div");
        let late = element(&mut doc, holder, "span", "");
        doc.set_attribute(late, "id", "late");
        doc.set_attribute(late, "hidden", "");
        text(&mut doc, late, "Later");
        holder
    };
    step(&doc, &mut model);

    // When: the subtree is inserted, its root alone noted
    doc.borrow_mut().append(row, holder);
    let update = step(&doc, &mut model).expect("the insert");

    // Then
    assert!(
        labels(&update.nodes).contains(&"button \"Later\"".to_owned()),
        "{:?}",
        labels(&update.nodes)
    );
    assert_eq!(scans(&doc), 0);
}

#[test]
fn a_hidden_element_taking_an_id_names_its_referrer_without_a_scan() {
    // Given: After labelled by "late"; a hidden span with no id, which
    // has no box, so no box touches it
    let (doc, mut model) = taken_model();
    let after = node_of(&doc, "button", "After");
    let row = doc.borrow().parent(after).expect("the row");
    let holder = {
        let mut doc = doc.borrow_mut();
        doc.set_attribute(after, "aria-labelledby", "late");
        hidden_label(&mut doc, row, "Later")
    };
    let late = doc.borrow().children(holder)[0];
    doc.borrow_mut().set_attribute(late, "id", "early");
    step(&doc, &mut model);

    // When: it takes the id in place
    doc.borrow_mut().set_attribute(late, "id", "late");
    let update = step(&doc, &mut model).expect("a rename");

    // Then
    assert!(
        labels(&update.nodes).contains(&"button \"Later\"".to_owned()),
        "{:?}",
        labels(&update.nodes)
    );
    assert_eq!(scans(&doc), 0);
}

#[test]
fn a_hidden_element_removed_with_an_id_unnames_its_referrer_without_a_scan() {
    // Given: After labelled by a hidden #late inside an inserted subtree
    let (doc, mut model) = taken_model();
    let after = node_of(&doc, "button", "After");
    let row = doc.borrow().parent(after).expect("the row");
    let holder = {
        let mut doc = doc.borrow_mut();
        doc.set_attribute(after, "aria-labelledby", "late");
        hidden_label(&mut doc, row, "Later")
    };
    step(&doc, &mut model).expect("named");

    // When: the subtree is removed
    doc.borrow_mut().remove(holder);
    let update = step(&doc, &mut model).expect("the removal");

    // Then: After is no longer named "Later", and the index was patched
    assert!(
        !labels(&update.nodes).contains(&"button \"Later\"".to_owned()),
        "{:?}",
        labels(&update.nodes)
    );
    assert!(
        update.nodes.iter().any(|node| node.node == after),
        "After is renamed: {:?}",
        labels(&update.nodes)
    );
    assert_eq!(scans(&doc), 0);
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
    doc.borrow_mut().set_attribute(first, "id", "was-lab");

    // When: the first gives up the id in place, which only a scan settles
    let update = step(&doc, &mut model).expect("a rename");

    // Then
    assert_eq!(labels(&update.nodes), ["button \"Second\""]);
    assert_eq!(scans(&doc), 1);
}
