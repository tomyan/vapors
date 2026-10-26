//! Focus (3-E2a): Tab and Shift+Tab move focus through the focusable
//! elements in tree order, and past the last focus leaves the page; a
//! press focuses the nearest focusable element; `:focus` and
//! `:focus-within` follow focus; a positive tabindex is refused.

mod support;

use std::cell::RefCell;
use std::rc::Rc;

use support::focus_page::{page, tab};
use support::{document, element, refusals};
use vapors_dom::{Dom, Event};
use vapors_elements::Document;

#[test]
fn tab_visits_the_focusable_elements_in_tree_order_then_leaves() {
    // Given
    let page = page();

    // When / Then: not c (disabled), d (tabindex -1) or the hidden one
    let mut visited = Vec::new();
    for _ in 0..4 {
        tab(&page.doc, false);
        visited.push(page.doc.borrow().active_element());
    }
    assert_eq!(
        visited,
        [Some(page.save), Some(page.note), Some(page.cancel), None]
    );
}

#[test]
fn shift_tab_from_outside_goes_to_the_last_then_back() {
    // Given
    let page = page();

    // When / Then
    tab(&page.doc, true);
    assert_eq!(page.doc.borrow().active_element(), Some(page.cancel));
    tab(&page.doc, true);
    assert_eq!(page.doc.borrow().active_element(), Some(page.note));
}

#[test]
fn focus_moves_with_blur_focusout_focus_focusin_in_order() {
    // Given: a focused, then Tab to b
    let page = page();
    tab(&page.doc, false);
    let log = Rc::new(RefCell::new(Vec::new()));
    for (node, name) in [(page.save, "a"), (page.note, "b")] {
        for event in ["blur", "focusout", "focus", "focusin"] {
            let (log, name) = (Rc::clone(&log), name.to_owned());
            page.doc.borrow_mut().listen(
                node,
                event,
                Box::new(move |e: &mut Event| {
                    log.borrow_mut().push(format!("{name} {}", e.name()));
                }),
            );
        }
    }

    // When
    tab(&page.doc, false);

    // Then: as Chromium orders them; focusout and focusin bubble, so b's
    // focusin is not heard on a
    assert_eq!(
        *log.borrow(),
        ["a blur", "a focusout", "b focus", "b focusin"]
    );
}

#[test]
fn a_press_focuses_the_element_or_blurs_to_nothing() {
    // Given: d (tabindex -1) at y 68..88 (the buttons above are 24 px)
    let page = page();

    // When / Then: a press on d focuses it; a press below everything blurs
    Document::pointer_pressed(&page.doc, [5.0, 75.0]);
    Document::pointer_released(&page.doc, [5.0, 75.0]);
    assert_eq!(page.doc.borrow().active_element(), Some(page.aside));
    Document::pointer_pressed(&page.doc, [5.0, 190.0]);
    assert_eq!(page.doc.borrow().active_element(), None);
}

#[test]
fn a_positive_tabindex_is_refused_by_name() {
    // Given
    let mut doc = document();
    let body = doc.body();
    let save = element(&mut doc, body, "div", "a");
    doc.set_attribute(save, "tabindex", "2");

    // When
    let errors = refusals(&mut doc);

    // Then
    let message = &errors[0];
    assert!(
        message.contains("tabindex=\"2\"") && message.contains("or -1"),
        "{message}"
    );
}

#[test]
fn focus_and_focus_within_style_the_focused_element_and_its_ancestors() {
    // Given: a sheet colouring :focus and :focus-within
    let page = page();
    page.doc
        .borrow_mut()
        .add_stylesheet("states.css", ".a:focus { background-color: #00ff00; } body:focus-within .b { background-color: #0000ff; }")
        .expect("parses");

    // When: Tab to the first button
    tab(&page.doc, false);
    page.doc
        .borrow_mut()
        .update(support::viewport(100.0, 200.0))
        .expect("updates");

    // Then: the button is drawn focused, and the body counts as holding focus
    let colours: Vec<[u8; 4]> = page
        .doc
        .borrow()
        .painted()
        .expect("painted")
        .scene()
        .items()
        .iter()
        .filter_map(|item| match item {
            vapors_scene::Item::Quad(quad) => Some(quad.background.to_rgba8()),
            _ => None,
        })
        .collect();
    assert!(
        colours.contains(&[0, 255, 0, 255]) && colours.contains(&[0, 0, 255, 255]),
        "{colours:?}"
    );
}

#[test]
fn focus_from_a_key_is_visible_and_from_a_press_is_not() {
    // Given
    let page = page();

    // When / Then: Tab to the first button
    tab(&page.doc, false);
    assert!(page.doc.borrow().focus_visible(), "after Tab");

    // a press on the note box focuses it without the ring
    Document::pointer_pressed(&page.doc, [5.0, 30.0]);
    Document::pointer_released(&page.doc, [5.0, 30.0]);
    assert_eq!(page.doc.borrow().active_element(), Some(page.note));
    assert!(!page.doc.borrow().focus_visible(), "after a press");

    // focus moved by script after the press: still no ring; after a key: the ring
    Document::focus(&page.doc, Some(page.cancel));
    assert!(!page.doc.borrow().focus_visible(), "script after a press");
    Document::key_pressed(&page.doc, "x", vapors_elements::Modifiers::default());
    Document::focus(&page.doc, Some(page.save));
    assert!(page.doc.borrow().focus_visible(), "script after a key");
}

#[test]
fn a_modifier_alone_is_not_a_keyboard_interaction() {
    // Given: focus from a press, then Shift alone
    let page = page();
    Document::pointer_pressed(&page.doc, [5.0, 30.0]);
    Document::key_pressed(&page.doc, "Shift", vapors_elements::Modifiers::default());

    // When: script moves focus
    Document::focus(&page.doc, Some(page.save));

    // Then: no ring, as a modifier alone does not count
    assert!(!page.doc.borrow().focus_visible());
}
