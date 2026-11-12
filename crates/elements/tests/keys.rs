//! Keys (3-E2a): keys go to the focused element, else the body; a
//! button is clicked by Enter on keydown and by Space on keyup; a
//! prevented keydown does neither, nor moves focus.

mod support;

use std::cell::RefCell;
use std::rc::Rc;

use support::focus_page::{page, tab};
use vapors_dom::{Dom, Event};
use vapors_elements::{Document, Modifiers};

#[test]
fn keys_go_to_the_focused_element_else_the_body() {
    // Given
    let page = page();
    let log = Rc::new(RefCell::new(Vec::new()));
    let body = page.doc.borrow().body();
    let into = Rc::clone(&log);
    page.doc.borrow_mut().listen(
        body,
        "keydown",
        Box::new(move |e: &mut Event| into.borrow_mut().push((e.target(), e.key().to_owned()))),
    );

    // When: x with nothing focused, then Tab (to a), then y
    Document::key_pressed(&page.doc, "x", Modifiers::default());
    tab(&page.doc, false);
    Document::key_pressed(&page.doc, "y", Modifiers::default());

    // Then
    assert_eq!(
        *log.borrow(),
        [
            (Some(body), "x".to_owned()),
            (Some(body), "Tab".to_owned()),
            (Some(page.save), "y".to_owned())
        ]
    );
}

#[test]
fn enter_clicks_a_button_on_keydown_and_space_on_keyup() {
    // Given: a focused, counting clicks with the phase of the key
    let page = page();
    tab(&page.doc, false);
    let clicks = Rc::new(RefCell::new(0));
    let into = Rc::clone(&clicks);
    page.doc.borrow_mut().listen(
        page.save,
        "click",
        Box::new(move |_: &mut Event| *into.borrow_mut() += 1),
    );

    // When / Then
    Document::key_pressed(&page.doc, "Enter", Modifiers::default());
    assert_eq!(*clicks.borrow(), 1, "Enter clicks on keydown");
    Document::key_released(&page.doc, "Enter", Modifiers::default());
    Document::key_pressed(&page.doc, " ", Modifiers::default());
    assert_eq!(*clicks.borrow(), 1, "Space waits for keyup");
    Document::key_released(&page.doc, " ", Modifiers::default());
    assert_eq!(*clicks.borrow(), 2);
}

#[test]
fn a_prevented_keydown_does_not_click_or_move_focus() {
    // Given: a focused; its keydown listener prevents the default
    let page = page();
    tab(&page.doc, false);
    page.doc.borrow_mut().listen(
        page.save,
        "keydown",
        Box::new(|e: &mut Event| e.prevent_default()),
    );
    let clicks = Rc::new(RefCell::new(0));
    let into = Rc::clone(&clicks);
    page.doc.borrow_mut().listen(
        page.save,
        "click",
        Box::new(move |_: &mut Event| *into.borrow_mut() += 1),
    );

    // When
    Document::key_pressed(&page.doc, "Enter", Modifiers::default());
    tab(&page.doc, false);

    // Then
    assert_eq!(*clicks.borrow(), 0);
    assert_eq!(page.doc.borrow().active_element(), Some(page.save));
}
