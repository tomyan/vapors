//! A host that drives its own frames calls `Document::settle_focus`
//! after each update; a key or typed text given while the focused
//! element is not focusable and no settle has run since the update is
//! refused, naming the call.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::{Dom, NodeId};
use vapors_elements::{Document, Focus};
use vapors_platform::{Event, Modifiers, Platform};

use super::key_tests::{VIEWPORT, card};
use crate::{DocumentInput, Unplaced};

/// The key, typed character and edited text a host gives.
fn typing() -> [Event; 3] {
    let key = Event::Key {
        key: "a".into(),
        code: "KeyA".into(),
        pressed: true,
        repeat: false,
        modifiers: Modifiers::default(),
    };
    let edited = Event::TextEdited {
        before: String::new(),
        text: "a".into(),
        selection: (1, 1),
        composing: None,
    };
    [key, Event::Character("a".into()), edited]
}

/// The card's button focused, then hidden and the document updated.
fn focused_then_hidden() -> (Rc<RefCell<Document>>, NodeId) {
    let (doc, button, _) = card();
    assert_eq!(Document::focus(&doc, Some(button)), Focus::Done);
    let mut document = doc.borrow_mut();
    document.set_attribute(button, "style", "display: none");
    document.update(VIEWPORT).expect("updates");
    drop(document);
    (doc, button)
}

fn give(doc: &Rc<RefCell<Document>>, event: &Event) -> Result<(), Unplaced> {
    let mut input = DocumentInput::default();
    input.give(doc, event, &Platform::detached()).map(|_| ())
}

#[test]
fn typing_after_an_update_without_settle_focus_is_refused_naming_it() {
    for event in typing() {
        // Given
        let (doc, _) = focused_then_hidden();

        // When
        let given = give(&doc, &event);

        // Then
        assert_eq!(given, Err(Unplaced::FocusNotSettled), "{event:?}");
        let message = Unplaced::FocusNotSettled.to_string();
        assert!(message.contains("settle_focus"), "{message}");
    }
}

#[test]
fn typing_after_settle_focus_is_given() {
    // Given
    let (doc, _) = focused_then_hidden();
    Document::settle_focus(&doc);

    // When
    let given = give(&doc, &typing()[0]);

    // Then
    assert_eq!(given, Ok(()));
    assert_eq!(doc.borrow().active_element(), None);
}

#[test]
fn a_key_after_a_hidden_button_is_shown_and_focused_is_given() {
    // Given: the button hidden at the last update, shown and focused since
    let (doc, button, _) = card();
    doc.borrow_mut()
        .set_attribute(button, "style", "display: none");
    doc.borrow_mut().update(VIEWPORT).expect("updates");
    doc.borrow_mut().set_attribute(button, "style", "");
    assert_eq!(Document::focus(&doc, Some(button)), Focus::Done);

    // When
    let given = give(&doc, &typing()[0]);

    // Then
    assert_eq!(given, Ok(()));
    assert_eq!(doc.borrow().active_element(), Some(button));
}
