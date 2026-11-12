//! The a11y update follows focus (R6): a focus move emits the focused
//! node and the records of the node that lost focus and the one that
//! gained it; a move to nothing emits `Some(None)`; a move refused
//! (a disabled button) emits nothing.

mod support;

use support::mirror::{labels, node_of, taken_page, updated_take};
use vapors_elements::Document;

#[test]
fn focus_gained_emits_the_focused_node_and_its_record() {
    // Given
    let doc = taken_page();
    let apply = node_of(&doc, "button", "Apply");
    Document::focus(&doc, Some(apply));

    // When
    let update = updated_take(&doc).expect("a change");

    // Then
    assert_eq!(update.focus, Some(Some(apply)));
    assert_eq!(labels(&update.nodes), ["button \"Apply\""]);
    assert!(update.nodes[0].states.focused);
}

#[test]
fn focus_moved_emits_both_records_and_the_new_node() {
    // Given
    let doc = taken_page();
    let apply = node_of(&doc, "button", "Apply");
    let pin = node_of(&doc, "button", "Pin");
    Document::focus(&doc, Some(apply));
    updated_take(&doc).expect("the first move");
    Document::focus(&doc, Some(pin));

    // When
    let update = updated_take(&doc).expect("a change");

    // Then
    assert_eq!(update.focus, Some(Some(pin)));
    assert_eq!(
        labels(&update.nodes),
        ["button \"Apply\"", "button \"Pin\""]
    );
    let focused = |name: &str| {
        update
            .nodes
            .iter()
            .find(|n| n.name == name)
            .expect(name)
            .states
            .focused
    };
    assert!(focused("Pin") && !focused("Apply"));
}

#[test]
fn focus_leaving_emits_no_node_focused() {
    // Given
    let doc = taken_page();
    let apply = node_of(&doc, "button", "Apply");
    Document::focus(&doc, Some(apply));
    updated_take(&doc).expect("the move in");
    Document::focus(&doc, None);

    // When
    let update = updated_take(&doc).expect("a change");

    // Then
    assert_eq!(update.focus, Some(None));
    assert_eq!(labels(&update.nodes), ["button \"Apply\""]);
    assert!(!update.nodes[0].states.focused);
}

#[test]
fn a_refused_focus_move_emits_nothing() {
    // Given: Reset is disabled
    let doc = taken_page();
    let reset = node_of(&doc, "button", "Reset");
    Document::focus(&doc, Some(reset));

    // When
    let update = updated_take(&doc);

    // Then
    assert!(update.is_none(), "{update:?}");
}
