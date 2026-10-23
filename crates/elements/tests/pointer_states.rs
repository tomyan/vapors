//! `:hover` and `:active` follow the pointer and ask for a frame only
//! when they change.

mod support;

use std::cell::RefCell;
use std::rc::Rc;

use support::pointer::{Log, SHEET, record, setup, update};
use vapors_dom::Dom;
use vapors_elements::Document;
use vapors_scene::Item;

#[test]
fn hover_and_active_restyle_the_chain_under_the_pointer() {
    // Given
    let sheet = "body { margin: 0; } .a, .b { height: 20px; } .a:hover { background-color: #00ff00; } .row:active { background-color: #ff0000; }";
    let (doc, row, a, _) = setup(sheet);
    doc.borrow_mut().take_changed();

    // When: hover .a, then press on it
    Document::pointer_moved(&doc, [5.0, 5.0]);
    let hover_changed = doc.borrow_mut().take_changed();
    Document::pointer_pressed(&doc, [5.0, 5.0]);
    update(&doc);

    // Then
    assert!(hover_changed, "a hover change asks for a frame");
    let colours: Vec<[u8; 4]> = doc
        .borrow()
        .painted()
        .expect("painted")
        .scene()
        .items()
        .iter()
        .filter_map(|item| match item {
            Item::Quad(quad) => Some(quad.background.to_rgba8()),
            Item::Text { .. } | Item::Shadow(_) => None,
        })
        .collect();
    assert_eq!(
        colours,
        [[255, 0, 0, 255], [0, 255, 0, 255]],
        "row :active, then .a :hover"
    );
    let _ = (row, a);
}

#[test]
fn leaving_the_window_clears_hover() {
    // Given
    let sheet =
        "body { margin: 0; } .a, .b { height: 20px; } .a:hover { background-color: #00ff00; }";
    let (doc, _, _, _) = setup(sheet);
    Document::pointer_moved(&doc, [5.0, 5.0]);

    // When
    Document::pointer_left(&doc);
    update(&doc);

    // Then
    assert_eq!(doc.borrow().painted().expect("painted").scene().items(), []);
}

#[test]
fn moving_within_the_hovered_element_changes_nothing() {
    // Given
    let (doc, _, _, _) = setup(SHEET);
    Document::pointer_moved(&doc, [5.0, 5.0]);
    doc.borrow_mut().take_changed();

    // When
    Document::pointer_moved(&doc, [6.0, 7.0]);

    // Then
    assert!(!doc.borrow_mut().take_changed());
}

fn quad_colours(doc: &Rc<RefCell<Document>>) -> Vec<[u8; 4]> {
    doc.borrow()
        .painted()
        .expect("painted")
        .scene()
        .items()
        .iter()
        .filter_map(|item| match item {
            Item::Quad(quad) => Some(quad.background.to_rgba8()),
            Item::Text { .. } => None,
        })
        .collect()
}

#[test]
fn hover_follows_a_layout_change_under_a_still_pointer() {
    // Given: the pointer rests on .a (y 0..20); .b is below it
    let sheet = "body { margin: 0; } .a, .b { height: 20px; } .b:hover { background-color: #00ff00; } .gone { display: none; }";
    let (doc, _, a, _) = setup(sheet);
    Document::pointer_moved(&doc, [5.0, 5.0]);

    // When: .a disappears, so .b moves under the pointer; two updates,
    // the first moving :hover, the second showing it
    doc.borrow_mut().set_attribute(a, "class", "a gone");
    update(&doc);
    let asked = doc.borrow_mut().take_changed();
    update(&doc);

    // Then
    assert!(asked, "the hover change asks for a frame");
    assert_eq!(quad_colours(&doc), [[0, 255, 0, 255]]);
}

#[test]
fn leaving_the_window_mid_press_clears_active_and_clicks_nothing() {
    // Given
    let sheet =
        "body { margin: 0; } .a, .b { height: 20px; } .a:active { background-color: #ff0000; }";
    let (doc, _, a, _) = setup(sheet);
    let log = Log::default();
    record(&doc, a, "click", "a", &log);
    Document::pointer_moved(&doc, [5.0, 5.0]);
    Document::pointer_pressed(&doc, [5.0, 5.0]);

    // When
    Document::pointer_left(&doc);
    Document::pointer_moved(&doc, [5.0, 5.0]);
    Document::pointer_released(&doc, [5.0, 5.0]);
    update(&doc);

    // Then
    assert_eq!(*log.borrow(), Vec::<String>::new());
    assert_eq!(quad_colours(&doc), Vec::<[u8; 4]>::new());
}
