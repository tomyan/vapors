//! `:hover` and `:active` follow the pointer and ask for a frame only
//! when they change.

mod support;

use support::pointer::{SHEET, setup, update};
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
            Item::Text { .. } => None,
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
