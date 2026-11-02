//! The window losing and regaining focus, as headed Chromium 155 orders
//! it (probed under Xvfb by bringing another window to the front and
//! back; `crates/elements-corpus/probes/window-focus.mjs`): a number or
//! text field commits, then the focused element gets `blur` and
//! `focusout` and stays the active element; on return it gets `focus`
//! and `focusin`.

mod support;

use std::rc::Rc;

use support::number_page::{click, key, page};
use vapors_dom::{Dom, Event};
use vapors_elements::Document;

/// `page.amount`'s focus events, added to the page's input/change log.
fn log_focus(page: &support::number_page::Page) {
    for name in ["blur", "focusout", "focus", "focusin"] {
        let log = Rc::clone(&page.log);
        page.doc.borrow_mut().listen(
            page.amount,
            name,
            Box::new(move |e: &mut Event| log.borrow_mut().push(format!("amount {}", e.name()))),
        );
    }
}

#[test]
fn the_window_losing_focus_commits_then_blurs_and_keeps_the_active_element() {
    // Given: 20 edited to 2 in amount
    let page = page();
    click(&page, [50.0, 12.0]);
    log_focus(&page);
    key(&page, "Backspace");

    // When
    Document::window_focus(&page.doc, false);

    // Then
    assert_eq!(
        *page.log.borrow(),
        [
            "amount input \"2\" 2",
            "amount change \"2\" 2",
            "amount blur",
            "amount focusout"
        ]
    );
    assert_eq!(page.doc.borrow().active_element(), Some(page.amount));
}

#[test]
fn the_window_regaining_focus_focuses_the_element_again_once() {
    // Given: the window lost focus with amount focused
    let page = page();
    click(&page, [50.0, 12.0]);
    log_focus(&page);
    Document::window_focus(&page.doc, false);
    page.log.borrow_mut().clear();

    // When: back, then a repeated notice
    Document::window_focus(&page.doc, true);
    Document::window_focus(&page.doc, true);

    // Then
    assert_eq!(*page.log.borrow(), ["amount focus", "amount focusin"]);
}

#[test]
fn a_repeated_loss_or_one_with_nothing_focused_fires_nothing() {
    // Given
    let (focused, unfocused) = (page(), page());
    click(&focused, [50.0, 12.0]);
    log_focus(&focused);
    log_focus(&unfocused);

    // When
    Document::window_focus(&focused.doc, false);
    focused.log.borrow_mut().clear();
    Document::window_focus(&focused.doc, false);
    Document::window_focus(&unfocused.doc, false);
    Document::window_focus(&unfocused.doc, true);

    // Then
    assert!(
        focused.log.borrow().is_empty(),
        "{:?}",
        focused.log.borrow()
    );
    assert!(
        unfocused.log.borrow().is_empty(),
        "{:?}",
        unfocused.log.borrow()
    );
}

#[test]
fn the_window_losing_focus_commits_an_edited_text_field_then_blurs_it() {
    // Given: "Warm" edited to "Warms", its blur logged
    let page = support::text_page::page();
    support::text_page::click(&page, [150.0, 12.0]);
    let log = Rc::clone(&page.log);
    page.doc.borrow_mut().listen(
        page.name,
        "blur",
        Box::new(move |_: &mut Event| log.borrow_mut().push("blur".into())),
    );
    support::text_page::type_text(&page, "s");
    page.log.borrow_mut().clear();

    // When
    Document::window_focus(&page.doc, false);

    // Then: the change before the blur, as Chromium (the probe's #t)
    let log = page.log.borrow();
    let names: Vec<&str> = log.iter().filter_map(|l| l.split(' ').next()).collect();
    assert_eq!(names, ["change", "blur"], "{log:?}");
}
