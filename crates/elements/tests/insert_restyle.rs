//! Inserting or removing a child restyles only what a selector can
//! reach (splice.md (b2)): the inserted subtree, and among the siblings
//! only those a structural pseudo-class or sibling combinator in the
//! sheets can reach from the change. Counted on html > body > column >
//! 10 rows of 5 cells (63 elements); a row with its cells is 6.

mod support;

use support::column_page::{ALL, Page, page, restyled_after};
use vapors_dom::{Dom, NodeId};

const ROW: usize = 6;

/// A bare `div.row` inserted before `rows[at]`, or appended at 10.
fn insert_row(p: &mut Page, at: usize) -> NodeId {
    let row = p.doc.create_element("div");
    p.doc.set_attribute(row, "class", "row");
    let column = p.doc.parent(p.rows[0]).expect("rows are in the column");
    p.doc.insert_before(column, row, p.rows.get(at).copied());
    row
}

#[test]
fn without_structural_rules_an_insert_restyles_the_new_row_only() {
    // Given
    let mut page = page(".row { color: red; }");

    // When
    let count = restyled_after(&mut page, |p| drop(insert_row(p, 5)));

    // Then
    assert_eq!(count, 1);
}

#[test]
fn a_next_sibling_combinator_reaches_the_next_row() {
    // Given
    let mut page = page(".row + .row { margin-top: 1px; }");

    // When
    let inserted = restyled_after(&mut page, |p| drop(insert_row(p, 5)));
    let removed = restyled_after(&mut page, |p| p.doc.remove(p.rows[2]));

    // Then: the new row and the next; after a removal the next alone
    assert_eq!((inserted, removed), (2, 1));
}

#[test]
fn a_chain_of_next_sibling_combinators_reaches_as_many_rows() {
    // Given
    let mut page = page(".row + .row + .row { margin-top: 1px; }");

    // When
    let count = restyled_after(&mut page, |p| drop(insert_row(p, 5)));

    // Then: the new row and the next two
    assert_eq!(count, 3);
}

#[test]
fn a_next_sibling_rule_on_an_ancestor_reaches_the_next_rows_subtree() {
    // Given
    let mut page = page(".row + .row .cell { color: red; }");

    // When
    let count = restyled_after(&mut page, |p| drop(insert_row(p, 5)));

    // Then: the new row, and the next row with its cells
    assert_eq!(count, 1 + ROW);
}

#[test]
fn nth_child_reaches_the_rows_after() {
    // Given
    let mut page = page(".row:nth-child(odd) { color: red; }");

    // When
    let count = restyled_after(&mut page, |p| drop(insert_row(p, 5)));

    // Then: the new row, and rows 5 to 9 whose parity flipped, with their cells
    assert_eq!(count, 1 + 5 * ROW);
}

#[test]
fn last_child_reaches_the_old_and_new_last() {
    // Given
    let mut page = page(".row:last-child { color: red; }");

    // When
    let appended = restyled_after(&mut page, |p| drop(insert_row(p, 10)));
    let middle = restyled_after(&mut page, |p| drop(insert_row(p, 5)));
    let removed = restyled_after(&mut page, |p| {
        let last = p.doc.children(p.doc.parent(p.rows[0]).expect("column"))[11];
        p.doc.remove(last);
    });

    // Then: the old last changed, so its cells too, and the new row; in the
    // middle the previous row is restyled unchanged; the new last after a removal
    assert_eq!((appended, middle, removed), (ROW + 1, 2, 1));
}

#[test]
fn subsequent_sibling_combinator_reaches_the_following_rows() {
    // Given
    let mut page = page(".on ~ .row { color: red; }");

    // When
    let count = restyled_after(&mut page, |p| drop(insert_row(p, 5)));

    // Then: the new row and the 5 rows after it, each unchanged
    assert_eq!(count, 1 + 5);
}

#[test]
fn has_restyles_everything() {
    // Given
    let mut page = page(".column:has(.on) { color: red; }");

    // When
    let count = restyled_after(&mut page, |p| drop(insert_row(p, 5)));

    // Then
    assert_eq!(count, ALL + 1);
}

#[test]
fn a_sheet_added_between_inserts_changes_the_reach() {
    // Given
    let mut page = page(".row { color: red; }");
    let before = restyled_after(&mut page, |p| drop(insert_row(p, 5)));

    // When
    page.doc
        .add_stylesheet("more.css", ".row + .row { margin-top: 1px; }")
        .expect("parses");
    page.doc
        .update(support::viewport(400.0, 600.0))
        .expect("updates");
    let after = restyled_after(&mut page, |p| drop(insert_row(p, 5)));

    // Then
    assert_eq!((before, after), (1, 2));
}

#[test]
fn an_inserted_text_node_reaches_no_sibling() {
    // Given
    let mut page = page(".row + .row { margin-top: 1px; }");

    // When
    let count = restyled_after(&mut page, |p| {
        let text = p.doc.create_text("x");
        let column = p.doc.parent(p.rows[0]).expect("column");
        p.doc.insert_before(column, text, Some(p.rows[5]));
    });

    // Then
    assert_eq!(count, 0);
}
