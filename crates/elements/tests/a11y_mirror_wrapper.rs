//! An element that is not a node, wrapping nodes (R6, review 230-H3):
//! moved among its siblings, the nodes it holds are placed again even
//! when their boxes stayed (absolutely positioned, the shape of popovers
//! and toasts); hidden by `aria-hidden`, they are removed and come back
//! when it is shown; given a role, it takes them. Proven by the model
//! mirror held against the document after every take.

mod support;

use std::cell::RefCell;
use std::rc::Rc;

use support::element;
use support::mirror::{node_of, step, taken_model};
use support::roles_page::button;
use vapors_dom::{Dom, NodeId};
use vapors_elements::Document;

/// An absolutely positioned div under the region holding button
/// "Floating", with its first take applied.
fn floating(doc: &Rc<RefCell<Document>>) -> NodeId {
    let region = node_of(doc, "region", "Options");
    let mut doc = doc.borrow_mut();
    doc.add_stylesheet(
        "abs.css",
        ".abs { position: absolute; display: flex; left: 300px; top: 200px; }",
    )
    .expect("parses");
    let wrap = element(&mut doc, region, "div", "abs");
    button(&mut doc, wrap, "Floating", &[]);
    wrap
}

#[test]
fn an_absolute_wrapper_moved_among_siblings_places_the_nodes_it_holds() {
    // Given
    let (doc, mut model) = taken_model();
    let wrap = floating(&doc);
    step(&doc, &mut model).expect("the wrapper's node");
    let region = node_of(&doc, "region", "Options");
    let pin = node_of(&doc, "button", "Pin");
    let lock = node_of(&doc, "checkbox", "Lock");
    let floating = node_of(&doc, "button", "Floating");
    doc.borrow_mut().insert_before(region, wrap, Some(pin));

    // When
    let update = step(&doc, &mut model).expect("a placement");

    // Then
    let placed: Vec<_> = update.placed.iter().map(|p| p.node).collect();
    assert_eq!(placed, [floating]);
    let order = model.children(Some(region));
    let at = |node| order.iter().position(|&n| n == node).expect("held");
    assert!(
        at(lock) < at(floating) && at(floating) < at(pin),
        "{order:?}"
    );
}

#[test]
fn an_aria_hidden_wrapper_removes_its_nodes_and_shows_them_again() {
    // Given
    let (doc, mut model) = taken_model();
    let wrap = floating(&doc);
    step(&doc, &mut model).expect("the wrapper's node");
    let floating = node_of(&doc, "button", "Floating");
    doc.borrow_mut().set_attribute(wrap, "aria-hidden", "true");

    // When
    let hidden = step(&doc, &mut model).expect("a removal");
    doc.borrow_mut().set_attribute(wrap, "aria-hidden", "false");
    let shown = step(&doc, &mut model).expect("a placement");

    // Then
    assert_eq!(hidden.removed, [floating]);
    assert_eq!(shown.placed[0].node, floating);
}

#[test]
fn a_wrapper_given_a_role_takes_the_nodes_it_holds() {
    // Given
    let (doc, mut model) = taken_model();
    let wrap = floating(&doc);
    step(&doc, &mut model).expect("the wrapper's node");
    let floating = node_of(&doc, "button", "Floating");
    doc.borrow_mut().set_attribute(wrap, "role", "toolbar");
    doc.borrow_mut().set_attribute(wrap, "aria-label", "Tools");

    // When
    step(&doc, &mut model).expect("a change");

    // Then
    assert_eq!(model.children(Some(wrap)), [floating]);
}
