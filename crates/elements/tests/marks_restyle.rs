//! Restyle marks (IU3 PR 1b): a restyle marks an element only when its
//! computed style changed or holds a calc. In each case a hover changes
//! a cell's used values: through a selector, while the hovered row's
//! computed style stays the same, or through what the cell inherits
//! from the row. Each matches a full update.

mod support;

use std::cell::RefCell;
use std::rc::Rc;

use support::oracle::assert_matches_full;
use support::{document, element, text, viewport};
use vapors_elements::Document;

/// A row holding a cell with text, styled by `css`, then hovered.
fn hovered_row(css: &str) -> Rc<RefCell<Document>> {
    let mut doc = document();
    let body = doc.body();
    let row = element(&mut doc, body, "div", "row");
    let cell = element(&mut doc, row, "div", "cell");
    text(&mut doc, cell, "Cell");
    doc.add_stylesheet("app.css", &format!(".row {{ height: 40px; }} {css}"))
        .expect("parses");
    doc.update(viewport(200.0, 100.0)).expect("updates");
    let doc = Rc::new(RefCell::new(doc));
    Document::pointer_moved(&doc, [100.0, 30.0]);
    doc
}

#[test]
fn a_hover_selector_that_restyles_only_a_descendant_matches_a_full_update() {
    // Given: the row's own computed style does not change on hover
    let doc = hovered_row(".cell { width: 50px; } .row:hover .cell { width: 120px; }");

    // Then
    assert_matches_full(&doc, viewport(200.0, 100.0), "descendant selector");
}

#[test]
fn a_hovered_font_size_reaches_a_cell_sized_in_em_as_in_a_full_update() {
    // Given
    let doc = hovered_row(
        ".row { font-size: 10px; } .cell { width: 5em; } .row:hover { font-size: 20px; }",
    );

    // Then
    assert_matches_full(&doc, viewport(200.0, 100.0), "em width");
}

#[test]
fn a_hovered_color_reaches_a_current_color_border_as_in_a_full_update() {
    // Given
    let doc = hovered_row(
        ".cell { border-width: 2px; border-style: solid; border-color: currentColor; } \
         .row:hover { color: #ff0000; }",
    );

    // Then
    assert_matches_full(&doc, viewport(200.0, 100.0), "currentColor border");
}

#[test]
fn a_hovered_color_reaches_the_cells_text_as_in_a_full_update() {
    // Given
    let doc = hovered_row(".row:hover { color: #ff0000; }");

    // Then
    assert_matches_full(&doc, viewport(200.0, 100.0), "inherited text colour");
}
