//! The window's focus goes to the document and the host alike: losing
//! it blurs the card's focused control, regaining it focuses it again.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::{Dom, Event as DomEvent};
use vapors_elements::Document;
use vapors_platform::{Event, Platform};

use super::Route;
use super::key_tests::card;
use crate::DocumentInput;

#[test]
fn the_window_losing_and_regaining_focus_blurs_and_refocuses_the_card_for_both() {
    // Given: the card's range focused, its focus events logged
    let (doc, _, range) = card();
    Document::focus(&doc, Some(range));
    let log = Rc::new(RefCell::new(Vec::new()));
    for name in ["blur", "focus"] {
        let log = Rc::clone(&log);
        doc.borrow_mut().listen(
            range,
            name,
            Box::new(move |e: &mut DomEvent| log.borrow_mut().push(e.name().to_owned())),
        );
    }
    let mut input = DocumentInput::new();

    // When
    let routes = [false, true].map(|focused| {
        input
            .give(&doc, &Event::WindowFocus { focused }, &Platform::detached())
            .expect("placed")
            .route
    });

    // Then
    assert_eq!(routes, [Route::Both, Route::Both]);
    assert_eq!(*log.borrow(), ["blur", "focus"]);
    assert_eq!(doc.borrow().active_element(), Some(range));
}
