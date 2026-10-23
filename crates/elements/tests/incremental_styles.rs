//! Restyle is scoped to what changed (F2c S7): each test counts the
//! elements an update restyles after one kind of change, on html >
//! body > column > 10 rows of 5 cells (63 elements).

mod support;

use std::cell::RefCell;
use std::rc::Rc;

use support::{document, element, text, viewport};
use vapors_dom::{Dom, NodeId};
use vapors_elements::Document;

const ALL: usize = 63;

struct Page {
    doc: Document,
    rows: Vec<NodeId>,
    cells: Vec<NodeId>,
    texts: Vec<NodeId>,
}

/// The page under `css`, updated once.
fn page(css: &str) -> Page {
    let mut doc = document();
    let body = doc.body();
    let column = element(&mut doc, body, "div", "column");
    let (mut rows, mut cells, mut texts) = (Vec::new(), Vec::new(), Vec::new());
    for r in 0..10 {
        let row = element(&mut doc, column, "div", "row");
        rows.push(row);
        for c in 0..5 {
            let cell = element(&mut doc, row, "div", "cell");
            texts.push(text(&mut doc, cell, &format!("{r}.{c}")));
            cells.push(cell);
        }
    }
    doc.add_stylesheet("app.css", css).expect("parses");
    doc.update(viewport(400.0, 600.0)).expect("updates");
    Page {
        doc,
        rows,
        cells,
        texts,
    }
}

/// Updates after `change` and counts the elements restyled.
fn restyled_after(page: &mut Page, change: impl FnOnce(&mut Page)) -> usize {
    change(page);
    page.doc.update(viewport(400.0, 600.0)).expect("updates");
    page.doc.elements_restyled()
}

fn set_class(page: &mut Page, node: NodeId, class: &str) {
    page.doc.set_attribute(node, "class", class);
}

#[test]
fn nothing_changed_restyles_nothing() {
    // Given
    let mut page = page(".cell { color: red; }");

    // When
    let count = restyled_after(&mut page, |_| {});

    // Then
    assert_eq!(count, 0);
}

#[test]
fn a_text_change_restyles_nothing() {
    // Given
    let mut page = page(".cell { color: red; }");

    // When
    let count = restyled_after(&mut page, |p| p.doc.set_text(p.texts[7], "x"));

    // Then
    assert_eq!(count, 0);
}

#[test]
fn a_class_change_restyles_the_element_and_its_subtree() {
    // Given
    let mut page = page(".cell { color: red; }");

    // When
    let cell = restyled_after(&mut page, |p| set_class(p, p.cells[7], "cell on"));
    let row = restyled_after(&mut page, |p| set_class(p, p.rows[2], "row on"));

    // Then: a cell alone; a row and its 5 cells
    assert_eq!((cell, row), (1, 6));
}

#[test]
fn with_a_sibling_combinator_a_class_change_restyles_the_parents_subtree() {
    // Given
    let mut page = page(".on + .cell { color: red; }");

    // When
    let count = restyled_after(&mut page, |p| set_class(p, p.cells[7], "cell on"));

    // Then: the cell's row and its 5 cells
    assert_eq!(count, 6);
}

#[test]
fn with_has_any_change_restyles_everything() {
    // Given
    let mut page = page(".row:has(.on) { color: red; }");

    // When
    let count = restyled_after(&mut page, |p| set_class(p, p.cells[7], "cell on"));

    // Then
    assert_eq!(count, ALL);
}

#[test]
fn an_inserted_element_is_restyled_alone_and_a_removal_restyles_nothing() {
    // Given
    let mut page = page(".cell { color: red; }");

    // When
    let added = restyled_after(&mut page, |p| {
        let cell = p.doc.create_element("div");
        p.doc.append(p.rows[3], cell);
    });
    let removed = restyled_after(&mut page, |p| p.doc.remove(p.cells[0]));

    // Then
    assert_eq!((added, removed), (1, 0));
}

#[test]
fn a_new_sheet_or_another_viewport_restyles_everything() {
    // Given
    let mut page = page(".cell { color: red; }");

    // When
    let sheet = restyled_after(&mut page, |p| {
        p.doc.add_stylesheet("more.css", ".row {}").expect("parses");
    });
    page.doc.update(viewport(390.0, 600.0)).expect("updates");
    let resized = page.doc.elements_restyled();

    // Then
    assert_eq!((sheet, resized), (ALL, ALL));
}

#[test]
fn hover_on_a_subject_compound_restyles_the_hovered_chain_not_the_page() {
    // Given: :hover only on the subject compound
    let Page { doc, cells, .. } = page(".cell { height: 10px; } .cell:hover { color: red; }");
    let doc = Rc::new(RefCell::new(doc));
    let rect = doc
        .borrow()
        .painted()
        .expect("painted")
        .border_box(cells[7])
        .expect("a box");

    // When: the pointer moves onto a cell
    Document::pointer_moved(&doc, [rect.x + 1.0, rect.y + 1.0]);
    doc.borrow_mut()
        .update(viewport(400.0, 600.0))
        .expect("updates");

    // Then: the cell and its 4 ancestors, whose :hover flipped
    assert_eq!(doc.borrow().elements_restyled(), 5);
}

#[test]
fn a_restyled_element_shows_its_new_style() {
    // Given
    let mut page = page(".cell { height: 2px; } .on { background-color: #ff0000; }");

    // When
    let cell = page.cells[7];
    set_class(&mut page, cell, "cell on");
    let painted = page.doc.update(viewport(400.0, 600.0)).expect("updates");

    // Then: one red quad
    let red = vapors_scene::Color::from_rgba8(255, 0, 0, 255);
    let quads = painted
        .scene()
        .items()
        .iter()
        .filter(|item| matches!(item, vapors_scene::Item::Quad(q) if q.background == red))
        .count();
    assert_eq!(quads, 1);
}
