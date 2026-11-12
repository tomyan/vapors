//! A name taken from another element by id follows that element (R6,
//! review 230-H2): when the `aria-labelledby` target is removed or its
//! id changes, the referrer's name is read again in the same take; when
//! an element takes the id, likewise. Proven by the model mirror held
//! against the document after every take.

mod support;

use std::cell::RefCell;
use std::rc::Rc;

use support::mirror::{labels, node_of, step, taken_model};
use support::{element, text};
use vapors_dom::{Dom, NodeId};
use vapors_elements::Document;

/// A span `#lab` "Later" in After's row, and After labelled by it.
fn labelled_by(doc: &Rc<RefCell<Document>>) -> NodeId {
    let after = node_of(doc, "button", "After");
    let unnamed = doc.borrow().parent(after).expect("the row");
    let mut doc = doc.borrow_mut();
    let label = element(&mut doc, unnamed, "span", "");
    doc.set_attribute(label, "id", "lab");
    text(&mut doc, label, "Later");
    doc.set_attribute(after, "aria-labelledby", "lab");
    label
}

#[test]
fn a_referrer_takes_its_name_from_the_target() {
    // Given
    let (doc, mut model) = taken_model();
    labelled_by(&doc);

    // When
    let update = step(&doc, &mut model).expect("a change");

    // Then
    assert_eq!(labels(&update.nodes), ["button \"Later\""]);
}

#[test]
fn a_removed_target_gives_the_referrer_its_own_name_back() {
    // Given
    let (doc, mut model) = taken_model();
    let label = labelled_by(&doc);
    step(&doc, &mut model).expect("the label");
    doc.borrow_mut().remove(label);

    // When
    let update = step(&doc, &mut model).expect("a change");

    // Then
    assert_eq!(labels(&update.nodes), ["button \"After\""]);
}

#[test]
fn a_target_whose_id_changed_gives_the_referrer_its_own_name_back() {
    // Given
    let (doc, mut model) = taken_model();
    let label = labelled_by(&doc);
    step(&doc, &mut model).expect("the label");
    doc.borrow_mut().set_attribute(label, "id", "other");

    // When
    let update = step(&doc, &mut model).expect("a change");

    // Then
    assert_eq!(labels(&update.nodes), ["button \"After\""]);
}

#[test]
fn an_element_that_takes_the_id_names_the_referrer() {
    // Given: After refers to an id no element has yet
    let (doc, mut model) = taken_model();
    let after = node_of(&doc, "button", "After");
    let unnamed = doc.borrow().parent(after).expect("the row");
    doc.borrow_mut()
        .set_attribute(after, "aria-labelledby", "lab");
    step(&doc, &mut model);
    let span = {
        let mut doc = doc.borrow_mut();
        let span = element(&mut doc, unnamed, "span", "");
        text(&mut doc, span, "Later");
        span
    };
    step(&doc, &mut model);
    doc.borrow_mut().set_attribute(span, "id", "lab");

    // When
    let update = step(&doc, &mut model).expect("a change");

    // Then
    assert_eq!(labels(&update.nodes), ["button \"Later\""]);
}
