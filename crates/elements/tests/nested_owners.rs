//! An owner inside a shallow owner's kept child, rebuilt in the same
//! update as the shallow owner, in either slot order: the layout above
//! the nested owner is laid out again (review-v217 finding 1). Each
//! case matches a full update.

mod support;

use std::cell::RefCell;
use std::rc::Rc;

use support::oracle::assert_matches_full;
use support::{document, element, viewport};
use vapors_dom::{Dom, NodeId};
use vapors_elements::{Document, Viewport};

const CSS: &str = ".y { height: 10px; } .tall { height: 90px; }";

const VIEW: fn() -> Viewport = || viewport(400.0, 300.0);

/// Which slot the nested owner holds against the shallow owner `s`.
#[derive(Clone, Copy, Debug)]
enum Slot {
    Lower,
    Higher,
}

/// `body > s > k > y`, updated, then moved into place and updated; the nested element (`k` or `y`,
/// by `nested`) holds a slot lower or higher than `s`'s.
fn page(slot: Slot, nested: &str) -> (Document, NodeId, NodeId, NodeId) {
    let mut doc = document();
    let body = doc.body();
    let early = matches!(slot, Slot::Lower).then(|| element(&mut doc, body, "div", nested));
    let s = element(&mut doc, body, "div", "s");
    let k = if nested == "k" { early } else { None };
    let k = k.unwrap_or_else(|| element(&mut doc, s, "div", "k"));
    let y = if nested == "y" { early } else { None };
    let y = y.unwrap_or_else(|| element(&mut doc, k, "div", "y"));
    doc.add_stylesheet("n.css", CSS).expect("parses");
    doc.update(VIEW()).expect("updates");
    doc.append(s, k); // a taken key keeps its slot
    doc.append(k, y);
    doc.update(VIEW()).expect("updates");
    (doc, s, k, y)
}

/// Changes `s`'s children with no net difference, so `s` is a shallow
/// owner whose children match the last update's.
fn churn(doc: &mut Document, s: NodeId) {
    let extra = doc.create_element("div");
    doc.append(s, extra);
    doc.remove(extra);
}

fn assert_full(doc: Document, context: &str) {
    let doc = Rc::new(RefCell::new(doc));
    doc.borrow_mut().update(VIEW()).expect("updates");
    assert_matches_full(&doc, VIEW(), context);
}

#[test]
fn nested_owner_with_lower_slot_than_shallow_owner() {
    // Given: body > [y, s > k]; y holds a lower slot than s
    let mut doc = document();
    let body = doc.body();
    let y = element(&mut doc, body, "div", "y");
    let s = element(&mut doc, body, "div", "s");
    let k = element(&mut doc, s, "div", "k");
    doc.add_stylesheet("n.css", CSS).expect("parses");
    doc.update(VIEW()).expect("updates");
    doc.append(k, y); // y keeps its slot
    doc.update(VIEW()).expect("updates");

    // When: y grows, and s's children change with no net difference
    doc.set_attribute(y, "class", "y tall");
    churn(&mut doc, s);

    // Then
    assert_full(doc, "nested owner, lower slot");
}

#[test]
fn a_restyled_element_in_a_kept_child_relays_out_its_ancestors_in_either_slot_order() {
    for slot in [Slot::Lower, Slot::Higher] {
        // Given
        let (mut doc, s, _, y) = page(slot, "y");

        // When: y is a whole-subtree owner inside s's kept child k
        doc.set_attribute(y, "class", "y tall");
        churn(&mut doc, s);

        // Then
        assert_full(doc, &format!("restyled nested owner, {slot:?} slot"));
    }
}

#[test]
fn a_kept_child_whose_children_changed_relays_out_its_ancestors_in_either_slot_order() {
    for slot in [Slot::Lower, Slot::Higher] {
        // Given
        let (mut doc, s, k, _) = page(slot, "k");

        // When: k is a shallow owner inside s, which keeps it
        element(&mut doc, k, "div", "y tall");
        churn(&mut doc, s);

        // Then
        assert_full(doc, &format!("nested shallow owner, {slot:?} slot"));
    }
}
