//! A flag restyle that changes an inherited property reaches the
//! children (F2c S7): the hovered element is restyled alone, and its
//! children because its style changed.

mod support;

use std::cell::RefCell;
use std::rc::Rc;

use support::{document, element, text, viewport};
use vapors_elements::Document;
use vapors_scene::{Color, Item};

#[test]
fn hovering_a_row_whose_hover_sets_color_turns_its_cells_text_red() {
    // Given: .row:hover sets an inherited colour; the cells set none
    let mut doc = document();
    let body = doc.body();
    let row = element(&mut doc, body, "div", "row");
    let cell = element(&mut doc, row, "div", "cell");
    text(&mut doc, cell, "Cell");
    doc.add_stylesheet(
        "app.css",
        ".row { height: 40px; } .row:hover { color: #ff0000; }",
    )
    .expect("parses");
    doc.update(viewport(200.0, 100.0)).expect("updates");
    let doc = Rc::new(RefCell::new(doc));

    // When
    Document::pointer_moved(&doc, [100.0, 30.0]);
    let mut doc = doc.borrow_mut();
    let painted = doc.update(viewport(200.0, 100.0)).expect("updates");

    // Then: the cell's text is red
    let inks: Vec<Color> = painted
        .scene()
        .items()
        .iter()
        .filter_map(|item| match item {
            Item::Text { text, .. } => Some(painted.text(*text).runs()[0].color),
            _ => None,
        })
        .collect();
    assert_eq!(inks, [Color::from_rgba8(255, 0, 0, 255)]);
}
