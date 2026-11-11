//! `Document::focus` decides focusability from styles brought up to
//! date for the element and its ancestors, without layout, as
//! Chromium's `focus()` forces a style update: a field unhidden and
//! focused in one handler takes focus; a hidden, invisible or disabled
//! one does not, and the call says so ([`Focus`]). Chromium
//! 155.0.8059.79 (`node scripts/chromium-focus-fixup.mjs`, "blur
//! refocuses the field"): a `blur` handler's `focus()` on the field
//! just hidden does nothing and the body keeps focus.

mod support;

use std::rc::Rc;

use support::fixup::{hide_field, typed_in, update_and_settle};
use support::text_page::page;
use vapors_dom::{Dom, Event};
use vapors_elements::{Document, Focus};

#[test]
fn a_field_unhidden_and_focused_in_one_handler_takes_focus() {
    // Given: the field hidden, and focus gone from it
    let page = typed_in();
    hide_field(&page);
    update_and_settle(&page);

    // When: shown again and focused, with no update between
    page.doc
        .borrow_mut()
        .set_attribute(page.name, "style", "display: block");
    let result = Document::focus(&page.doc, Some(page.name));

    // Then
    assert_eq!(result, Focus::Done);
    assert_eq!(page.doc.borrow().active_element(), Some(page.name));
}

#[test]
fn focus_refuses_a_field_hidden_since_the_last_update() {
    // Given
    let page = page();
    hide_field(&page);

    // When
    let result = Document::focus(&page.doc, Some(page.name));

    // Then
    assert_eq!(result, Focus::NotFocusable);
    assert_eq!(page.doc.borrow().active_element(), None);
}

#[test]
fn focus_refuses_hidden_invisible_and_disabled_fields() {
    let cases = [
        ("body", "display: none", "an ancestor's display: none"),
        ("field", "display: block; visibility: hidden", "visibility"),
        ("disabled", "", "disabled"),
    ];
    for (target, style, why) in cases {
        // Given
        let page = page();
        let node = if target == "disabled" {
            page.off
        } else {
            page.name
        };
        let styled = if target == "body" {
            page.doc.borrow().body()
        } else {
            node
        };
        page.doc.borrow_mut().set_attribute(styled, "style", style);

        // When
        let result = Document::focus(&page.doc, Some(node));

        // Then
        assert_eq!(result, Focus::NotFocusable, "{why}");
        assert_eq!(page.doc.borrow().active_element(), None, "{why}");
    }
}

#[test]
fn a_blur_handler_refocusing_the_hidden_field_is_refused_and_the_page_goes_idle() {
    // Given: a blur handler that focuses the field again
    let page = typed_in();
    let (doc, field, log) = (Rc::clone(&page.doc), page.name, Rc::clone(&page.log));
    let refocus = move |_: &mut Event| {
        let result = Document::focus(&doc, Some(field));
        log.borrow_mut().push(format!("refocus {result:?}"));
    };
    page.doc
        .borrow_mut()
        .listen(page.name, "blur", Box::new(refocus));

    // When: hidden, then two frames
    hide_field(&page);
    update_and_settle(&page);
    let _ = page.doc.borrow_mut().take_changed();
    update_and_settle(&page);

    // Then: blurred once, refused, and nothing changed in the second frame
    assert_eq!(page.doc.borrow().active_element(), None);
    assert_eq!(
        *page.log.borrow(),
        [
            "change \"erWarm\"",
            "blur",
            "refocus NotFocusable",
            "focusout"
        ]
    );
    assert!(!page.doc.borrow_mut().take_changed(), "the page goes idle");
}

#[test]
fn focus_before_any_update_is_unresolved() {
    // Given: a field in a document never updated, so without a viewport
    let mut doc = support::document();
    let body = doc.body();
    let field = doc.create_element("input");
    doc.append(body, field);
    let doc = Rc::new(std::cell::RefCell::new(doc));

    // When
    let result = Document::focus(&doc, Some(field));

    // Then
    assert_eq!(result, Focus::Unresolved);
    assert_eq!(doc.borrow().active_element(), None);
}
