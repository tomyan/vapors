//! The a11y update when a node changes parent or place (R6): one that
//! lost its role hands its children to its parent, one that gained a
//! role takes its children from its parent, one moved among its
//! siblings is placed again, and one made `visibility: hidden` is
//! removed while its children stay, under its parent. Proven by the
//! model mirror (`support::mirror::Model`).

mod support;

use support::element;
use support::mirror::{labels, node_of, step, taken_model};
use vapors_dom::Dom;

#[test]
fn a_node_that_lost_its_role_hands_its_children_to_its_parent() {
    // Given: the group becomes presentational; its three controls move
    // up under the dialog, before the region
    let (doc, mut model) = taken_model();
    let dialog = node_of(&doc, "dialog", "Levels");
    let group = node_of(&doc, "group", "Tone");
    let region = node_of(&doc, "region", "Options");
    let after = node_of(&doc, "button", "After");
    let controls = [
        node_of(&doc, "slider", "Amount"),
        node_of(&doc, "spinbutton", "Radius"),
        node_of(&doc, "textbox", "Note"),
    ];
    doc.borrow_mut().set_attribute(group, "role", "none");

    // When
    let update = step(&doc, &mut model).expect("a change");

    // Then: a record carries its parent, so each control's record changed
    assert_eq!(update.removed, [group]);
    assert_eq!(
        labels(&update.nodes),
        [
            "slider \"Amount\"",
            "spinbutton \"Radius\"",
            "textbox \"Note\""
        ]
    );
    assert!(update.placed.iter().all(|p| p.parent == Some(dialog)));
    let mut expected = controls.to_vec();
    expected.extend([region, after]);
    assert_eq!(model.children(Some(dialog)), expected);
}

#[test]
fn a_node_that_gained_a_role_takes_its_children_from_its_parent() {
    // Given: a row with no role around Pin and Apply becomes a group
    let (doc, mut model) = taken_model();
    let region = node_of(&doc, "region", "Options");
    let pin = node_of(&doc, "button", "Pin");
    let apply = node_of(&doc, "button", "Apply");
    let reset = node_of(&doc, "button", "Reset");
    let wrap = {
        let mut doc = doc.borrow_mut();
        let wrap = element(&mut doc, region, "div", "row");
        doc.insert_before(region, wrap, Some(pin));
        doc.append(wrap, pin);
        doc.append(wrap, apply);
        wrap
    };
    step(&doc, &mut model).expect("the wrap's placements");
    doc.borrow_mut().set_attribute(wrap, "role", "group");
    doc.borrow_mut().set_attribute(wrap, "aria-label", "Edit");

    // When
    let update = step(&doc, &mut model).expect("a change");

    // Then
    assert_eq!(
        labels(&update.nodes),
        ["button \"Apply\"", "button \"Pin\"", "group \"Edit\""]
    );
    let mono = node_of(&doc, "checkbox", "Mono");
    let lock = node_of(&doc, "checkbox", "Lock");
    assert_eq!(model.children(Some(region)), [mono, lock, wrap, reset]);
    assert_eq!(model.children(Some(wrap)), [pin, apply]);
}

#[test]
fn a_node_moved_among_its_siblings_is_placed_again() {
    // Given: Reset moved before Pin; no record changes but its bounds
    let (doc, mut model) = taken_model();
    let region = node_of(&doc, "region", "Options");
    let pin = node_of(&doc, "button", "Pin");
    let reset = node_of(&doc, "button", "Reset");
    doc.borrow_mut().insert_before(region, reset, Some(pin));

    // When
    let update = step(&doc, &mut model).expect("a change");

    // Then
    let placed: Vec<_> = update.placed.iter().map(|p| p.node).collect();
    assert_eq!(placed, [reset]);
    let mono = node_of(&doc, "checkbox", "Mono");
    let lock = node_of(&doc, "checkbox", "Lock");
    let apply = node_of(&doc, "button", "Apply");
    assert_eq!(
        model.children(Some(region)),
        [mono, lock, reset, pin, apply]
    );
}

#[test]
fn a_node_made_invisible_is_removed_and_its_children_stay_under_its_parent() {
    // Given: the group hidden by visibility, its controls visible again
    let (doc, mut model) = taken_model();
    let dialog = node_of(&doc, "dialog", "Levels");
    let group = node_of(&doc, "group", "Tone");
    let region = node_of(&doc, "region", "Options");
    let after = node_of(&doc, "button", "After");
    let controls = [
        node_of(&doc, "slider", "Amount"),
        node_of(&doc, "spinbutton", "Radius"),
        node_of(&doc, "textbox", "Note"),
    ];
    {
        let mut doc = doc.borrow_mut();
        doc.add_stylesheet(
            "dim.css",
            ".dim { visibility: hidden; } .dim > * { visibility: visible; }",
        )
        .expect("parses");
        doc.set_attribute(group, "class", "row dim");
    }

    // When
    let update = step(&doc, &mut model).expect("a change");

    // Then
    assert_eq!(update.removed, [group]);
    let mut expected = controls.to_vec();
    expected.extend([region, after]);
    assert_eq!(model.children(Some(dialog)), expected);
}
