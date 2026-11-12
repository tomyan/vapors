//! A listener that removes the event's target, or the focused element:
//! the default action and the follow-on events after the dispatch find
//! the node gone and stop, as Chromium carries on after a handler
//! detaches a node (what it then fires on the detached node, nothing in
//! the document can observe). Each case panicked at `arena.rs` before
//! (review-fable-input H1).

mod support;

use std::cell::RefCell;
use std::num::NonZeroU32;
use std::rc::Rc;

use support::fixup::{hide_field, typed_in, update_and_settle};
use support::focus_page::{page as focus_page, tab};
use support::pointer::{Log, SHEET, record, setup};
use support::range_page::page as range_page;
use support::text_page::{click, page as text_page, value};
use vapors_dom::{Dom, Event, NodeId};
use vapors_elements::{Document, Modifiers};

/// `event` on `node` removes `node`.
fn remove_on(doc: &Rc<RefCell<Document>>, node: NodeId, event: &str) {
    let d = Rc::clone(doc);
    doc.borrow_mut().listen(
        node,
        event,
        Box::new(move |_: &mut Event| {
            if d.borrow().contains(node) {
                d.borrow_mut().remove(node);
            }
        }),
    );
}

#[test]
fn a_change_handler_that_removes_the_field_during_the_fixup_blurs_to_nothing() {
    // Given: the field's change removes it
    let page = typed_in();
    remove_on(&page.doc, page.name, "change");

    // When
    hide_field(&page);
    update_and_settle(&page);

    // Then: the change fired; blur and focusout had no node to go to
    assert_eq!(*page.log.borrow(), ["change \"erWarm\""]);
    assert_eq!(page.doc.borrow().active_element(), None);
}

#[test]
fn a_pointerdown_handler_that_removes_its_target_clicks_nothing() {
    // Given
    let (doc, row, a, _b) = setup(SHEET);
    let log = Log::default();
    record(&doc, row, "click", "row", &log);
    remove_on(&doc, a, "pointerdown");

    // When
    Document::pointer_pressed(&doc, [5.0, 5.0]);
    Document::pointer_released(&doc, [5.0, 5.0]);

    // Then: no click, as Chromium drops the click when the mousedown
    // node is removed
    assert!(!doc.borrow().contains(a));
    assert!(log.borrow().is_empty(), "{:?}", log.borrow());
}

#[test]
fn a_pointerup_handler_that_removes_its_target_clicks_nothing() {
    // Given: pressed on b, released over a, whose pointerup removes it
    let (doc, row, a, _b) = setup(SHEET);
    let log = Log::default();
    record(&doc, row, "click", "row", &log);
    remove_on(&doc, a, "pointerup");

    // When
    Document::pointer_pressed(&doc, [5.0, 25.0]);
    Document::pointer_released(&doc, [5.0, 5.0]);

    // Then
    assert!(!doc.borrow().contains(a));
    assert!(log.borrow().is_empty(), "{:?}", log.borrow());
}

#[test]
fn a_click_handler_that_removes_its_target_on_the_second_press_sends_no_dblclick() {
    // Given
    let (doc, row, a, _b) = setup(SHEET);
    let log = Log::default();
    record(&doc, row, "dblclick", "row", &log);
    remove_on(&doc, a, "click");

    // When
    Document::pointer_pressed_clicks(&doc, [5.0, 5.0], NonZeroU32::new(2).unwrap());
    Document::pointer_released(&doc, [5.0, 5.0]);

    // Then
    assert!(!doc.borrow().contains(a));
    assert!(log.borrow().is_empty(), "{:?}", log.borrow());
}

#[test]
fn a_keydown_handler_that_removes_the_focused_element_leaves_the_key_unused() {
    // Given
    let page = focus_page();
    tab(&page.doc, false);
    assert_eq!(page.doc.borrow().active_element(), Some(page.save));
    remove_on(&page.doc, page.save, "keydown");

    // When
    let used = Document::key_pressed(&page.doc, "Enter", Modifiers::default());

    // Then: no default action ran on the removed button
    assert!(!used);
    assert_eq!(page.doc.borrow().active_element(), None);
}

#[test]
fn a_keyup_handler_that_removes_the_focused_button_sends_no_click() {
    // Given: Space down on the button, whose keyup removes it
    let page = focus_page();
    tab(&page.doc, false);
    let log = Log::default();
    record(&page.doc, page.save, "click", "save", &log);
    remove_on(&page.doc, page.save, "keyup");
    Document::key_pressed(&page.doc, " ", Modifiers::default());

    // When
    Document::key_released(&page.doc, " ", Modifiers::default());

    // Then
    assert!(!page.doc.borrow().contains(page.save));
    assert!(log.borrow().is_empty(), "{:?}", log.borrow());
}

#[test]
fn a_taps_focus_handler_that_removes_the_tapped_element_clicks_nothing() {
    // Given
    let page = focus_page();
    let log = Log::default();
    let body = page.doc.borrow().body();
    record(&page.doc, body, "click", "body", &log);
    remove_on(&page.doc, page.save, "focus");

    // When
    Document::touch_pressed(&page.doc, 0, [5.0, 5.0]);
    Document::touch_released(&page.doc, 0, [5.0, 5.0]);
    Document::touch_tapped(&page.doc, [5.0, 5.0]);

    // Then: no click, as after a removed mousedown node
    assert!(!page.doc.borrow().contains(page.save));
    assert!(log.borrow().is_empty(), "{:?}", log.borrow());
}

#[test]
fn a_range_input_handler_that_removes_the_range_fires_no_change() {
    // Given: the range focused by a press, its `input` removing it
    let page = range_page();
    Document::tapped(&page.doc, [100.0, 10.0]);
    page.log.borrow_mut().clear();
    remove_on(&page.doc, page.level, "input");

    // When
    Document::key_pressed(&page.doc, "ArrowRight", Modifiers::default());

    // Then: the input fired; the change had no node to go to
    assert_eq!(page.log.borrow().len(), 1, "{:?}", page.log.borrow());
    assert!(!page.doc.borrow().contains(page.level));
}

#[test]
fn a_paste_handler_that_removes_the_field_pastes_nothing() {
    // Given
    let page = text_page();
    click(&page, [1.0, 12.0]);
    remove_on(&page.doc, page.name, "paste");

    // When
    let taken = Document::text_pasted(&page.doc, "er");

    // Then: the field took the paste event, and is gone
    assert!(taken);
    assert!(!page.doc.borrow().contains(page.name));
    assert_eq!(value(&page, page.plain), "");
}
