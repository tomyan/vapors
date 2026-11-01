//! Pasted text in a text field (E6b): `paste` fires on the field first
//! and a listener may cancel it; else the text replaces the selection
//! with `input`, as its own undo step, a line break a space. With no
//! text field focused the paste is not the document's, and the app has
//! it.

mod support;

use std::cell::RefCell;
use std::rc::Rc;

use support::text_page::{click, control, key_with, page, selection, type_text, value};
use vapors_dom::{Dom, Event};
use vapors_elements::Document;

#[test]
fn a_paste_replaces_the_selection_after_its_paste_event() {
    // Given: "Warm" selected whole by Tab
    let page = page();
    support::text_page::key(&page, "Tab");
    page.log.borrow_mut().clear();

    // When
    let taken = Document::text_pasted(&page.doc, "Cool");

    // Then
    assert!(taken);
    assert_eq!(value(&page, page.name), "Cool");
    assert_eq!(selection(&page, page.name), (4, 4));
    assert_eq!(*page.log.borrow(), ["paste \"\"", "input \"Cool\""]);
}

#[test]
fn a_cancelled_paste_inserts_nothing() {
    // Given: a listener that cancels the paste
    let page = page();
    click(&page, [120.0, 12.0]);
    page.doc.borrow_mut().listen(
        page.name,
        "paste",
        Box::new(|e: &mut Event| e.prevent_default()),
    );

    // When
    let taken = Document::text_pasted(&page.doc, "th");

    // Then: the field had the paste, and kept its text
    assert!(taken);
    assert_eq!(value(&page, page.name), "Warm");
}

#[test]
fn a_paste_with_no_text_field_focused_is_the_apps() {
    // Given: a button below the fields, focused by a click
    let page = page();
    let button = {
        let mut doc = page.doc.borrow_mut();
        let body = doc.body();
        let button = doc.create_element("button");
        doc.append(body, button);
        doc.add_stylesheet(
            "button.css",
            "button { display: block; width: 40px; height: 20px; }",
        )
        .expect("parses");
        doc.update(support::viewport(400.0, 100.0))
            .expect("updates");
        button
    };
    click(&page, [10.0, 80.0]);
    let heard = Rc::new(RefCell::new(0));
    let into = Rc::clone(&heard);
    page.doc.borrow_mut().listen(
        button,
        "paste",
        Box::new(move |_: &mut Event| *into.borrow_mut() += 1),
    );

    // When
    let taken = Document::text_pasted(&page.doc, "th");

    // Then
    assert_eq!(page.doc.borrow().active_element(), Some(button));
    assert!(!taken);
    assert_eq!(*heard.borrow(), 0);
}

#[test]
fn a_paste_is_an_undo_step_of_its_own() {
    // Given: "th" typed, then "X" pasted, then "y" typed
    let page = page();
    click(&page, [120.0, 12.0]);
    type_text(&page, "th");
    Document::text_pasted(&page.doc, "X");
    type_text(&page, "y");

    // When
    key_with(&page, "z", control());
    let without_y = value(&page, page.name);
    key_with(&page, "z", control());

    // Then
    assert_eq!(without_y, "WarmthX");
    assert_eq!(value(&page, page.name), "Warmth");
}

#[test]
fn a_disabled_field_takes_no_paste() {
    // Given: the disabled field pressed (it takes the caret, not focus)
    let page = page();
    click(&page, [150.0, 60.0]);

    // When
    let taken = Document::text_pasted(&page.doc, "x");

    // Then
    assert!(!taken);
    assert_eq!(value(&page, page.off), "off");
}

#[test]
fn each_line_break_in_a_paste_is_a_space() {
    // Given
    let page = page();
    click(&page, [120.0, 12.0]);

    // When
    Document::text_pasted(&page.doc, "a\nb\r\nc\rd");

    // Then
    assert_eq!(value(&page, page.name), "Warma b c d");
}
