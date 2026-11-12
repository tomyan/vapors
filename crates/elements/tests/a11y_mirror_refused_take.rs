//! A refused take is its own state, not "no change" (R6, review 230-H1):
//! what it held is held for the next take, so a change made in the same
//! take as the cause of a refusal reaches the mirror once the cause is
//! gone. And focus on an element the tree leaves out is refused, named
//! (230-M1): a reader would hear nothing.

mod support;

use std::cell::RefCell;
use std::rc::Rc;

use support::mirror::{labels, node_of, step, taken_model};
use support::{element, text, viewport};
use vapors_dom::{Dom, NodeId};
use vapors_elements::{AxError, Document};

/// Updates `doc` and takes; the refusal.
fn refused_take(doc: &Rc<RefCell<Document>>) -> AxError {
    let mut doc = doc.borrow_mut();
    doc.update(viewport(640.0, 400.0)).expect("updates");
    doc.take_ax_update().expect_err("refused")
}

/// A `radio` with no checked state under the region: refused.
fn bare_radio(doc: &Rc<RefCell<Document>>, region: NodeId) -> NodeId {
    let mut doc = doc.borrow_mut();
    let radio = element(&mut doc, region, "div", "");
    doc.set_attribute(radio, "role", "radio");
    doc.set_attribute(radio, "aria-label", "Mono2");
    text(&mut doc, radio, "x");
    radio
}

#[test]
fn a_refused_take_keeps_its_changes_for_the_next() {
    // Given: Pin pressed in the same take as a radio that is refused
    let (doc, mut model) = taken_model();
    let pin = node_of(&doc, "button", "Pin");
    let region = node_of(&doc, "region", "Options");
    doc.borrow_mut().set_attribute(pin, "aria-pressed", "true");
    let radio = bare_radio(&doc, region);
    let refused = refused_take(&doc);
    assert!(matches!(refused, AxError::Unexposable { .. }), "{refused}");
    doc.borrow_mut().remove(radio);

    // When: the cause is gone
    let update = step(&doc, &mut model).expect("Pin's change");

    // Then
    assert!(labels(&update.nodes).contains(&"button \"Pin\"".to_owned()));
    let pressed = update.nodes.iter().find(|n| n.node == pin).expect("Pin");
    assert_eq!(pressed.states.pressed, Some(true));
}

#[test]
fn a_refused_take_keeps_a_focus_move_for_the_next() {
    // Given: focus moved to Apply in the same take as a refused radio
    let (doc, mut model) = taken_model();
    let apply = node_of(&doc, "button", "Apply");
    let region = node_of(&doc, "region", "Options");
    Document::focus(&doc, Some(apply));
    let radio = bare_radio(&doc, region);
    refused_take(&doc);
    doc.borrow_mut().remove(radio);

    // When
    let update = step(&doc, &mut model).expect("the focus move");

    // Then
    assert_eq!(update.focus, Some(Some(apply)));
}

#[test]
fn focus_on_a_focusable_element_with_no_role_is_refused_naming_it() {
    // Given: a tabindex div with no role, which the tree leaves out
    let (doc, mut model) = taken_model();
    let after = node_of(&doc, "button", "After");
    let unnamed = doc.borrow().parent(after).expect("the row");
    let div = {
        let mut doc = doc.borrow_mut();
        let div = element(&mut doc, unnamed, "div", "");
        doc.set_attribute(div, "tabindex", "0");
        text(&mut doc, div, "focus me");
        div
    };
    step(&doc, &mut model);
    Document::focus(&doc, Some(div));

    // When
    let refused = refused_take(&doc);

    // Then
    match refused {
        AxError::FocusOutsideTree { tag } => assert_eq!(tag, "div"),
        other => panic!("not refused as focus outside the tree: {other:?}"),
    }
}
