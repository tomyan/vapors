//! Focus cases from review-v81: `visibility: hidden` is not focusable,
//! Tab continues from a focused element outside the Tab order, removing
//! the focused element clears focus, Space clicks on its own keyup only,
//! and Tab with Control, Alt or Meta does not move focus.

mod support;

use std::cell::RefCell;
use std::rc::Rc;

use support::focus_page::{page, tab};
use support::{document, element, viewport};
use vapors_dom::{Dom, Event, NodeId};
use vapors_elements::{Document, Modifiers};
use vapors_scene::Item;

fn shared(doc: Document) -> Rc<RefCell<Document>> {
    Rc::new(RefCell::new(doc))
}

#[test]
fn a_hidden_element_is_not_focusable_but_a_visible_child_of_one_is() {
    // Given: a hidden button; a hidden box holding a visible button
    let mut doc = document();
    let body = doc.body();
    let hidden = element(&mut doc, body, "button", "hidden");
    let parent = element(&mut doc, body, "div", "parent");
    let inside = element(&mut doc, parent, "button", "inside");
    let shown = element(&mut doc, parent, "button", "shown");
    doc.add_stylesheet(
        "app.css",
        "body { margin: 0; } button, div { display: block; height: 20px; } .hidden { visibility: hidden; } .parent { visibility: hidden; height: auto; } .shown { visibility: visible; }",
    )
    .expect("parses");
    doc.update(viewport(100.0, 200.0)).expect("updates");
    let doc = shared(doc);
    let _ = (hidden, inside);

    // When: Tab, and a press on the hidden button
    tab(&doc, false);
    let tabbed = doc.borrow().active_element();
    tab(&doc, false);
    Document::pointer_pressed(&doc, [5.0, 5.0]);

    // Then: only the visible child takes focus, and the press blurs
    assert_eq!(tabbed, Some(shown));
    assert_eq!(doc.borrow().active_element(), None);
}

#[test]
fn tab_from_an_element_outside_the_tab_order_goes_on_from_its_place() {
    // Given: the aside (tabindex -1) focused by a press
    let page = page();
    Document::focus(&page.doc, Some(page.aside));

    // When / Then: Tab to Cancel after it; Shift+Tab to the note before it
    tab(&page.doc, false);
    assert_eq!(page.doc.borrow().active_element(), Some(page.cancel));
    Document::focus(&page.doc, Some(page.aside));
    tab(&page.doc, true);
    assert_eq!(page.doc.borrow().active_element(), Some(page.note));
}

#[test]
fn removing_the_focused_element_clears_focus_and_focus_within() {
    // Given: Save focused, the body styled while it holds focus
    let page = page();
    page.doc
        .borrow_mut()
        .add_stylesheet(
            "states.css",
            "body:focus-within .b { background-color: #0000ff; }",
        )
        .expect("parses");
    tab(&page.doc, false);

    // When
    page.doc.borrow_mut().remove(page.save);

    // Then: nothing focused, the body no longer holds focus, Tab starts over
    assert_eq!(page.doc.borrow().active_element(), None);
    page.doc
        .borrow_mut()
        .update(viewport(100.0, 200.0))
        .expect("updates");
    let blue = page
        .doc
        .borrow()
        .painted()
        .expect("painted")
        .scene()
        .items()
        .iter()
        .any(|item| matches!(item, Item::Quad(q) if q.background.to_rgba8() == [0, 0, 255, 255]));
    assert!(!blue, ":focus-within still matches the body");
    tab(&page.doc, false);
    assert_eq!(page.doc.borrow().active_element(), Some(page.note));
}

fn clicks_on(doc: &Rc<RefCell<Document>>, node: NodeId) -> Rc<RefCell<u32>> {
    let clicks = Rc::new(RefCell::new(0));
    let into = Rc::clone(&clicks);
    doc.borrow_mut().listen(
        node,
        "click",
        Box::new(move |_: &mut Event| *into.borrow_mut() += 1),
    );
    clicks
}

#[test]
fn space_clicks_on_its_own_keyup_through_another_keys_tap() {
    // Given: Save focused
    let page = page();
    tab(&page.doc, false);
    let clicks = clicks_on(&page.doc, page.save);
    let none = Modifiers::default();

    // When: Space down, Shift tapped, Space up
    Document::key_pressed(&page.doc, " ", none);
    Document::key_pressed(&page.doc, "Shift", none);
    Document::key_released(&page.doc, "Shift", none);
    Document::key_released(&page.doc, " ", none);

    // Then
    assert_eq!(*clicks.borrow(), 1);
}

#[test]
fn space_released_after_focus_moved_clicks_nothing() {
    // Given: Space down on Save, then focus moves to Cancel
    let page = page();
    tab(&page.doc, false);
    let (save, cancel) = (
        clicks_on(&page.doc, page.save),
        clicks_on(&page.doc, page.cancel),
    );
    Document::key_pressed(&page.doc, " ", Modifiers::default());
    Document::focus(&page.doc, Some(page.cancel));

    // When
    Document::key_released(&page.doc, " ", Modifiers::default());

    // Then
    assert_eq!((*save.borrow(), *cancel.borrow()), (0, 0));
}

#[test]
fn tab_with_control_alt_or_meta_does_not_move_focus() {
    // Given
    let page = page();
    tab(&page.doc, false);

    // When / Then
    for modifiers in [
        Modifiers {
            control: true,
            ..Modifiers::default()
        },
        Modifiers {
            alt: true,
            ..Modifiers::default()
        },
        Modifiers {
            meta: true,
            ..Modifiers::default()
        },
    ] {
        Document::key_pressed(&page.doc, "Tab", modifiers);
        Document::key_released(&page.doc, "Tab", modifiers);
        assert_eq!(
            page.doc.borrow().active_element(),
            Some(page.save),
            "{modifiers:?}"
        );
    }
}

#[test]
fn a_prevented_pointerdown_does_not_focus() {
    // Given: Save's pointerdown listener prevents the default, as
    // Chromium 155 answers (gpu_events_a_prevented_pointerdown_does_not_focus_in_chromium)
    let page = page();
    page.doc.borrow_mut().listen(
        page.save,
        "pointerdown",
        Box::new(|e: &mut Event| e.prevent_default()),
    );

    // When: a press on Save
    Document::pointer_pressed(&page.doc, [5.0, 5.0]);

    // Then
    assert_eq!(page.doc.borrow().active_element(), None);
}
