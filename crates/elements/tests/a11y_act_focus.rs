//! Typing and keys stay on the node they were sent to (R1): a listener
//! that moves focus mid-way stops them, and the refusal names the node
//! that took focus and how many keys went before.

mod support;

use std::cell::RefCell;
use std::rc::{Rc, Weak};

use support::drive::{log, node, shared, target};
use support::roles_page::page;
use vapors_dom::{Dom, Event, NodeId};
use vapors_elements::{AxError, Document, Modifiers};

/// On `event` at `on`, focus moves to `to`, as a page's script may.
fn moves_focus(doc: &Rc<RefCell<Document>>, on: NodeId, event: &str, to: NodeId) {
    let weak: Weak<RefCell<Document>> = Rc::downgrade(doc);
    let moved = move |_: &mut Event| {
        let doc = weak.upgrade().expect("the document");
        Document::focus(&doc, Some(to));
    };
    doc.borrow_mut().listen(on, event, Box::new(moved));
}

/// Focus lost from `role` `name` to `button "Apply"` after `sent` of `of`.
fn lost(role: &str, name: &str, sent: usize, of: usize) -> AxError {
    AxError::FocusLost {
        role: role.to_owned(),
        name: name.to_owned(),
        to: Some("button \"Apply\"".to_owned()),
        sent,
        of,
    }
}

#[test]
fn typing_stops_when_a_listener_moves_focus_and_names_where_it_went() {
    // Given: the first keyup in Note moves focus to Apply
    let doc = shared(page());
    let (note, apply) = (node(&doc, "textbox", "Note"), node(&doc, "button", "Apply"));
    moves_focus(&doc, note, "keyup", apply);
    let clicks = log(&doc, apply, &["click"]);

    // When
    let typed = Document::ax_type(&doc, &target("textbox", "Note"), "yo");

    // Then: one key went to Note, none to Apply
    assert_eq!(typed, Err(lost("textbox", "Note", 1, 2)));
    assert_eq!(doc.borrow().value(note).as_deref(), Some("yhi"));
    assert!(clicks.borrow().is_empty());
}

#[test]
fn a_key_is_not_sent_when_focusing_the_node_moved_focus_away() {
    // Given: focusing Amount moves focus to Apply
    let doc = shared(page());
    let (amount, apply) = (
        node(&doc, "slider", "Amount"),
        node(&doc, "button", "Apply"),
    );
    moves_focus(&doc, amount, "focus", apply);

    // When
    let modifiers = Modifiers::default();
    let keyed = Document::ax_key(&doc, &target("slider", "Amount"), "ArrowRight", modifiers);

    // Then
    assert_eq!(keyed, Err(lost("slider", "Amount", 0, 1)));
    assert_eq!(doc.borrow().value(amount).as_deref(), Some("50"));
}
