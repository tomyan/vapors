//! The a11y update a mirror takes (R6, `docs/plans/r6.md`): every node
//! in document order, each with its parent, a slider with its range
//! and each node with whether it takes focus; a take before an update
//! is refused.

mod support;

use support::mirror::{node_of, taken_page};
use support::roles_page::page;
use vapors_dom::Dom;
use vapors_elements::{AxError, AxRange};

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
fn a_take_before_an_update_is_refused() {
    // Given
    let doc = taken_page();
    let pin = node_of(&doc, "button", "Pin");
    doc.borrow_mut().set_attribute(pin, "aria-pressed", "true");

    // When
    let refused = doc.borrow_mut().take_ax_update();

    // Then
    assert!(matches!(refused, Err(AxError::NotUpdated)), "{refused:?}");
}
