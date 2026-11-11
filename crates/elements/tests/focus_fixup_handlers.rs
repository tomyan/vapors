//! The focus fixup with handlers that move focus themselves, against
//! Chromium 155.0.8059.79 (`node scripts/chromium-focus-fixup.mjs`):
//! the field's `change` runs with focus already on the body, so a
//! handler that focuses a button keeps it focused: `change` on the
//! field, `focus` and `focusin` on the button, then `blur` and
//! `focusout` on the field, and a typed key reaches the button.

mod support;

use std::rc::Rc;

use support::fixup::{hide_field, typed_in, update_and_settle};
use support::text_page::Page;
use vapors_dom::{Dom, Event, NodeId};
use vapors_elements::Document;

/// A button after the fields, its `focus` and `focusin` logged.
fn button(page: &Page) -> NodeId {
    let mut doc = page.doc.borrow_mut();
    let button = doc.create_element("button");
    doc.set_attribute(button, "style", "display: block; height: 20px");
    let body = doc.body();
    doc.append(body, button);
    drop(doc);
    log_focus(page, button, "button");
    button
}

/// Logs `focus` and `focusin` on `node` as "focus on `label`".
fn log_focus(page: &Page, node: NodeId, label: &'static str) {
    for kind in ["focus", "focusin"] {
        let into = Rc::clone(&page.log);
        let handler =
            move |e: &mut Event| into.borrow_mut().push(format!("{} on {label}", e.name()));
        page.doc.borrow_mut().listen(node, kind, Box::new(handler));
    }
}

/// The page typed in, with a button the field's `change` focuses.
fn change_focuses_button() -> (Page, NodeId) {
    let page = typed_in();
    let button = button(&page);
    update_and_settle(&page);
    let doc = Rc::clone(&page.doc);
    let to_button = move |_: &mut Event| Document::focus(&doc, Some(button));
    page.doc
        .borrow_mut()
        .listen(page.name, "change", Box::new(to_button));
    (page, button)
}

/// Chromium's events when the field's `change` focuses the button.
const BUTTON_FOCUSED: [&str; 5] = [
    "change \"erWarm\"",
    "focus on button",
    "focusin on button",
    "blur",
    "focusout",
];

#[test]
fn a_change_handler_that_focuses_a_button_keeps_it_focused() {
    // Given
    let (page, button) = change_focuses_button();

    // When
    hide_field(&page);
    update_and_settle(&page);

    // Then: the field blurred once, the button kept
    assert_eq!(*page.log.borrow(), BUTTON_FOCUSED);
    assert_eq!(page.doc.borrow().active_element(), Some(button));
}

#[test]
fn focus_moving_to_a_field_stays_where_the_change_handler_put_it() {
    // Given
    let (page, button) = change_focuses_button();
    log_focus(&page, page.plain, "plain");

    // When
    Document::focus(&page.doc, Some(page.plain));

    // Then: the other field never gets focus or its events
    assert_eq!(*page.log.borrow(), BUTTON_FOCUSED);
    assert_eq!(page.doc.borrow().active_element(), Some(button));
}
