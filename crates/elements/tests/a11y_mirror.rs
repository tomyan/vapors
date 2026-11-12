//! The a11y update a mirror takes (R6, `docs/plans/r6.md`): the first
//! take holds every node in document order, each with its parent; after
//! that a take holds only the nodes whose record changed since the last,
//! and nothing when none did. A take before an update is refused.

mod support;

use support::mirror::{labels, node_of, step, taken_model};
use support::roles_page::page;
use support::viewport;
use vapors_dom::Dom;
use vapors_elements::{AxError, AxRange, Document};

#[test]
fn the_first_take_holds_every_node_in_document_order_with_its_parent() {
    // Given
    let mut doc = page();
    let flat: Vec<String> = doc
        .ax_projection(&[
            "dialog",
            "group",
            "slider",
            "spinbutton",
            "textbox",
            "region",
            "checkbox",
            "button",
        ])
        .expect("updated")
        .iter()
        .map(|node| format!("{} \"{}\"", node.role, node.name))
        .collect();

    // When
    let update = doc
        .take_ax_update()
        .expect("an updated document")
        .expect("the first take holds every node");

    // Then
    assert!(update.every);
    let in_order: Vec<String> = update
        .nodes
        .iter()
        .map(|node| format!("{} \"{}\"", node.role, node.name))
        .collect();
    assert_eq!(in_order, flat, "every node, a parent before its children");
    assert!(update.removed.is_empty() && update.placed.is_empty());
    let group = update.nodes[1].node;
    let slider = &update.nodes[2];
    assert_eq!(slider.parent, Some(group), "the slider is under the group");
    assert_eq!(update.nodes[0].parent, None, "the dialog is a top node");
    assert_eq!(update.focus, Some(None), "nothing has focus");
}

#[test]
fn a_slider_carries_its_range_and_a_button_its_focusability() {
    // Given
    let mut doc = page();

    // When
    let update = doc.take_ax_update().expect("updated").expect("every");

    // Then
    let slider = &update.nodes[2];
    assert_eq!(slider.role, "slider");
    assert_eq!(
        slider.range,
        Some(AxRange {
            min: 0.0,
            max: 100.0
        })
    );
    let by_name = |name: &str| update.nodes.iter().find(|n| n.name == name).expect(name);
    assert!(by_name("Apply").focusable);
    assert!(
        !by_name("Reset").focusable,
        "a disabled button takes no focus"
    );
    assert!(!by_name("Options").focusable, "a region takes no focus");
    assert_eq!(by_name("Options").range, None);
}

#[test]
fn a_text_change_under_a_button_emits_that_buttons_record_alone() {
    // Given
    let (doc, mut model) = taken_model();
    let apply = node_of(&doc, "button", "Apply");
    let label = doc.borrow().children(apply)[0];
    doc.borrow_mut().set_text(label, "Go");

    // When
    let update = step(&doc, &mut model).expect("a change");

    // Then
    assert!(!update.every);
    assert_eq!(labels(&update.nodes), ["button \"Go\""]);
    assert_eq!(update.nodes[0].node, apply);
    assert!(update.removed.is_empty() && update.placed.is_empty());
    assert_eq!(update.focus, None);
}

#[test]
fn an_aria_state_set_emits_one_record_with_the_state() {
    // Given
    let (doc, mut model) = taken_model();
    let pin = node_of(&doc, "button", "Pin");
    doc.borrow_mut().set_attribute(pin, "aria-pressed", "true");

    // When
    let update = step(&doc, &mut model).expect("a change");

    // Then
    assert_eq!(labels(&update.nodes), ["button \"Pin\""]);
    assert_eq!(update.nodes[0].states.pressed, Some(true));
}

#[test]
fn a_change_that_touches_nothing_a11y_emits_nothing() {
    // Given: an attribute no role, name, state or style reads
    let (doc, mut model) = taken_model();
    let group = node_of(&doc, "group", "Tone");
    doc.borrow_mut().set_attribute(group, "data-x", "1");

    // When
    let update = step(&doc, &mut model);

    // Then
    assert!(update.is_none(), "{update:?}");
}

#[test]
fn a_take_with_nothing_changed_since_the_last_is_none() {
    // Given
    let (doc, mut model) = taken_model();

    // When
    let update = step(&doc, &mut model);

    // Then
    assert!(update.is_none(), "{update:?}");
}

#[test]
fn a_take_before_an_update_is_refused() {
    // Given
    let (doc, _model) = taken_model();
    let pin = node_of(&doc, "button", "Pin");
    doc.borrow_mut().set_attribute(pin, "aria-pressed", "true");

    // When
    let refused = doc.borrow_mut().take_ax_update();

    // Then
    assert!(matches!(refused, Err(AxError::NotUpdated)), "{refused:?}");
}

#[test]
fn the_first_take_leaves_out_what_an_aria_hidden_wrapper_holds() {
    // Given: the group hidden from readers; the unnamed row is no node
    let mut doc = page();
    let find = |doc: &Document, role, name| doc.ax_find(role, name, None).expect(name).node;
    let group = find(&doc, "group", "Tone");
    let dialog = find(&doc, "dialog", "Levels");
    let after = find(&doc, "button", "After");
    doc.set_attribute(group, "aria-hidden", "true");
    doc.update(viewport(640.0, 400.0)).expect("updates");

    // When
    let update = doc.take_ax_update().expect("updated").expect("every");

    // Then
    let names: Vec<&str> = update.nodes.iter().map(|n| n.name.as_str()).collect();
    assert!(
        !names
            .iter()
            .any(|n| ["Tone", "Amount", "Radius", "Note"].contains(n)),
        "{names:?}"
    );
    let record = update
        .nodes
        .iter()
        .find(|n| n.node == after)
        .expect("After");
    assert_eq!(record.parent, Some(dialog), "through the row with no role");
}
