//! The index of ids a take reads names through, patched for elements
//! moved out of the document into a parent not in it, and back (4i, the
//! #432 review's H1): a move out drops the moved subtree's entries, as a
//! removal does, and a move back in puts them back, as an insert does;
//! neither scans. Every patched take is compared with a scan in debug
//! builds (`AxIndex::assert_scanned`).

mod support;

use std::cell::RefCell;
use std::rc::Rc;

use support::mirror::{labels, node_of, step, taken_model};
use support::mirror_model::Model;
use support::{element, text};
use vapors_dom::{Dom, NodeId};
use vapors_elements::Document;

/// Whether the last take scanned the document for its index.
fn scans(doc: &Rc<RefCell<Document>>) -> usize {
    doc.borrow().ax_take_counts().index_scans
}

/// The page with After labelled by a hidden span `#late` reading
/// "Later", in After's row: the document, After and the span.
fn labelled() -> (Rc<RefCell<Document>>, Model, NodeId, NodeId) {
    let (doc, mut model) = taken_model();
    let after = node_of(&doc, "button", "After");
    let row = doc.borrow().parent(after).expect("the row");
    let late = {
        let mut doc = doc.borrow_mut();
        doc.set_attribute(after, "aria-labelledby", "late");
        let late = element(&mut doc, row, "span", "");
        doc.set_attribute(late, "id", "late");
        doc.set_attribute(late, "hidden", "");
        text(&mut doc, late, "Later");
        late
    };
    step(&doc, &mut model).expect("named");
    (doc, model, after, late)
}

#[test]
fn an_element_with_a_referenced_id_moved_out_of_the_document_unnames_its_referrer() {
    // Given
    let (doc, mut model, after, late) = labelled();

    // When: the span is moved into a div not in the document
    {
        let mut doc = doc.borrow_mut();
        let apart = doc.create_element("div");
        doc.insert_before(apart, late, None);
    }
    let update = step(&doc, &mut model).expect("the move out");

    // Then: After is renamed, and the index was patched
    assert!(
        update.nodes.iter().any(|node| node.node == after),
        "{:?}",
        labels(&update.nodes)
    );
    assert!(!labels(&update.nodes).contains(&"button \"Later\"".to_owned()));
    assert_eq!(scans(&doc), 0);
}

#[test]
fn an_element_with_a_referenced_id_moved_out_and_back_names_its_referrer_again() {
    // Given: the span moved out, taken
    let (doc, mut model, after, late) = labelled();
    let row = doc.borrow().parent(after).expect("the row");
    {
        let mut doc = doc.borrow_mut();
        let apart = doc.create_element("div");
        doc.insert_before(apart, late, None);
    }
    step(&doc, &mut model).expect("the move out");

    // When: moved back into the row
    doc.borrow_mut().insert_before(row, late, None);
    let update = step(&doc, &mut model).expect("the move back");

    // Then
    assert!(
        labels(&update.nodes).contains(&"button \"Later\"".to_owned()),
        "{:?}",
        labels(&update.nodes)
    );
    assert_eq!(scans(&doc), 0);
}

#[test]
fn an_element_with_no_id_moved_out_of_the_document_is_dropped_from_the_index() {
    // Given
    let (doc, mut model) = taken_model();
    let pin = node_of(&doc, "button", "Pin");

    // When: a button with no id is moved into a div not in the document
    {
        let mut doc = doc.borrow_mut();
        let apart = doc.create_element("div");
        doc.insert_before(apart, pin, None);
    }
    step(&doc, &mut model).expect("the move out");

    // Then: the take's debug comparison with a scan held, without a scan
    assert_eq!(scans(&doc), 0);
}

#[test]
fn an_element_removed_and_another_inserted_with_its_id_in_one_update_scans_once() {
    // Given
    let (doc, mut model, after, late) = labelled();
    let row = doc.borrow().parent(after).expect("the row");

    // When: the span is removed and a new span with its id inserted, as a
    // keyed list re-rendering a row does, in one update
    {
        let mut doc = doc.borrow_mut();
        doc.remove(late);
        let again = element(&mut doc, row, "span", "");
        doc.set_attribute(again, "id", "late");
        doc.set_attribute(again, "hidden", "");
        text(&mut doc, again, "Again");
    }
    let update = step(&doc, &mut model).expect("the swap");

    // Then: After is named by the new span. A known scan: the freed span
    // still holds the id in the index when the new holder is tested
    // (`AxIndex::certain`), so the patch is not certain; recorded here so
    // a change to it shows in this count
    assert!(
        labels(&update.nodes).contains(&"button \"Again\"".to_owned()),
        "{:?}",
        labels(&update.nodes)
    );
    assert_eq!(scans(&doc), 1);
}
