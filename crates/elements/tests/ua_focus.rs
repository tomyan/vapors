//! The user-agent sheet's focus ring and button sizing (3-E2b): an
//! element focused by a key gets vapors' ring, one focused by a press
//! does not; a button's width includes its padding and border, as
//! Chromium's buttons are `box-sizing: border-box`.

mod support;

use support::focus_page::{page, tab};
use support::{document, element, viewport};
use vapors_elements::Document;
use vapors_scene::Item;

/// vapors' focus ring colour (the UA sheet's).
const RING: [u8; 4] = [0x00, 0x60, 0xdf, 0xff];

fn ring_drawn(doc: &std::rc::Rc<std::cell::RefCell<Document>>) -> bool {
    doc.borrow_mut()
        .update(viewport(100.0, 200.0))
        .expect("updates");
    doc.borrow()
        .painted()
        .expect("painted")
        .scene()
        .items()
        .iter()
        .any(|item| matches!(item, Item::Quad(quad) if quad.border_colors[0].to_rgba8() == RING && quad.border[0] > 0.0))
}

#[test]
fn a_key_focus_draws_the_ring_and_a_press_focus_does_not() {
    // Given
    let page = page();

    // When / Then
    tab(&page.doc, false);
    assert!(ring_drawn(&page.doc), "after Tab");
    Document::pointer_pressed(&page.doc, [5.0, 30.0]);
    assert!(!ring_drawn(&page.doc), "after a press");
}

#[test]
fn a_buttons_width_includes_its_padding_and_border() {
    // Given: a 100px button, in a flex row so it is laid out
    let mut doc = document();
    let body = doc.body();
    let row = element(&mut doc, body, "div", "row");
    let button = element(&mut doc, row, "button", "b");
    doc.add_stylesheet(
        "app.css",
        ".row { display: flex; } .b { width: 100px; padding: 0 10px; border: 2px solid black; }",
    )
    .expect("parses");

    // When
    let painted = doc.update(viewport(300.0, 100.0)).expect("updates");

    // Then
    assert_eq!(painted.border_box(button).expect("a box").width, 100.0);
}

#[test]
fn removing_the_focused_element_takes_its_ring_with_it() {
    // Given: Save focused by a key, drawing the ring
    let page = page();
    tab(&page.doc, false);
    assert!(ring_drawn(&page.doc));

    // When
    vapors_dom::Dom::remove(&mut *page.doc.borrow_mut(), page.save);

    // Then
    assert!(!page.doc.borrow().focus_visible());
    assert!(!ring_drawn(&page.doc));
}
