//! Inserting or removing a row builds only that row's boxes and the
//! list's own; the other rows keep their slots and kept boxes
//! (`docs/plans/splice.md`, section (b)). Each case also matches a
//! full update.

mod support;

use std::cell::RefCell;
use std::rc::Rc;

use support::oracle::assert_matches_full;
use support::{document, element, text, viewport};
use vapors_dom::{Dom, NodeId};
use vapors_elements::{Document, Viewport};

const ROWS: &str = ".row { display: flex; height: 20px; } .cell { width: 40px; }";

const VIEW: fn() -> Viewport = || viewport(400.0, 300.0);

struct List {
    doc: Rc<RefCell<Document>>,
    parent: NodeId,
    rows: Vec<NodeId>,
}

/// A row: `div.row > div.cell > text`, three boxes (row, cell, run).
fn row(doc: &mut Document, label: &str) -> NodeId {
    let row = doc.create_element("div");
    doc.set_attribute(row, "class", "row");
    let cell = element(doc, row, "div", "cell");
    text(doc, cell, label);
    row
}

/// `count` rows in a list under body, styled by `css`, updated once.
fn list(count: usize, css: &str) -> List {
    let mut doc = document();
    let body = doc.body();
    let list = element(&mut doc, body, "div", "list");
    let rows = (0..count)
        .map(|index| {
            let node = row(&mut doc, &format!("row {index}"));
            doc.append(list, node);
            node
        })
        .collect();
    doc.add_stylesheet("rows.css", css).expect("parses");
    doc.update(VIEW()).expect("updates");
    List {
        doc: Rc::new(RefCell::new(doc)),
        parent: list,
        rows,
    }
}

/// Updates and returns how many boxes the update built.
fn built(list: &List) -> usize {
    let mut doc = list.doc.borrow_mut();
    doc.update(VIEW()).expect("updates");
    doc.update_counts().boxes_built
}

/// Inserts a new row before row `at`.
fn insert_at(list: &List, at: usize) {
    let mut doc = list.doc.borrow_mut();
    let node = row(&mut doc, "new");
    doc.insert_before(list.parent, node, Some(list.rows[at]));
}

#[test]
fn inserting_a_row_in_the_middle_builds_only_its_boxes_and_the_lists() {
    // Given
    let list = list(20, ROWS);

    // When
    insert_at(&list, 10);

    // Then: the list's box, the row's, its cell's and its text run's
    assert_eq!(built(&list), 4);
    assert_matches_full(&list.doc, VIEW(), "insert in the middle");
}

#[test]
fn removing_a_row_builds_only_the_lists_box() {
    // Given
    let list = list(20, ROWS);

    // When
    list.doc.borrow_mut().remove(list.rows[10]);

    // Then
    assert_eq!(built(&list), 1);
    assert_matches_full(&list.doc, VIEW(), "remove in the middle");
}

#[test]
fn a_row_moved_within_the_list_keeps_every_row_box() {
    // Given
    let list = list(20, ROWS);

    // When: a keyed `{for}` reorder moves row 15 before row 3
    list.doc
        .borrow_mut()
        .insert_before(list.parent, list.rows[15], Some(list.rows[3]));

    // Then
    assert_eq!(built(&list), 1);
    assert_matches_full(&list.doc, VIEW(), "reorder");
}

#[test]
fn rows_restyled_by_their_position_are_rebuilt_after_an_insert() {
    for (css, rule) in [
        (".row:nth-child(odd) { height: 30px; }", "nth-child"),
        (".row:first-child { height: 30px; }", "first-child"),
        (".row + .row { margin-top: 3px; }", "next sibling"),
    ] {
        // Given
        let list = list(6, &format!("{ROWS} {css}"));

        // When
        insert_at(&list, 0);

        // Then: more than the new row's boxes and the list's
        assert!(built(&list) > 4, "{rule}");
        assert_matches_full(&list.doc, VIEW(), rule);
    }
}

#[test]
fn a_row_moved_to_another_list_is_built_there_and_taken_from_the_first() {
    // Given: two lists of five rows
    let first = list(5, ROWS);
    let second = {
        let mut doc = first.doc.borrow_mut();
        let body = doc.body();
        let second = element(&mut doc, body, "div", "list");
        doc.update(VIEW()).expect("updates");
        second
    };

    // When
    first.doc.borrow_mut().append(second, first.rows[2]);

    // Then: both lists' boxes and the moved row's three
    assert_eq!(built(&first), 5);
    assert_matches_full(&first.doc, VIEW(), "move between lists");
}

#[test]
fn an_absolute_cell_in_a_kept_row_is_placed_against_the_list_after_an_insert() {
    // Given: each row's cell placed against the list
    let css = ".list { position: relative; } .cell { position: absolute; top: 2px; }";
    let list = list(6, &format!("{ROWS} {css}"));

    // When
    insert_at(&list, 3);

    // Then
    assert_eq!(built(&list), 4);
    assert_matches_full(&list.doc, VIEW(), "absolute cells");
}
