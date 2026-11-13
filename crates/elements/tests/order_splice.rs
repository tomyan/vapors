//! Inserting or removing a row splices its steps into the kept paint
//! order in place of walking the whole tree (`docs/plans/splice.md`,
//! section (c)): `order_walks` and `chain_walks` 0, `order_splices` 1.
//! A debug build compares every spliced order with one walked afresh
//! (`kept_order_check.rs`); the small cases also match a full update
//! through the oracle. The declined cases: `order_splice_declined.rs`.

mod support;

use support::splice_page::{
    ROW_X, SPLICED, inserted, many_rows, matches_full, order, page, updated,
};
use vapors_dom::Dom;

#[test]
fn inserting_a_row_in_the_middle_of_many_splices_the_order() {
    // Given
    let rows = many_rows();
    let page = page(rows);

    // When
    let row = inserted(&page, Some(rows / 2), "");
    let counts = updated(&page);

    // Then: no walk, one splice, and the row is hit where it lies
    assert_eq!(order(&counts), SPLICED);
    let doc = page.doc.borrow();
    let painted = doc.painted().expect("painted");
    let y = painted.border_box(row).expect("the row has a box").y;
    assert_eq!(painted.hit([ROW_X, y + 2.0]), Some(row));
}

#[test]
fn removing_a_row_from_many_splices_the_order() {
    // Given
    let rows = many_rows();
    let page = page(rows);

    // When
    page.doc.borrow_mut().remove(page.rows[rows / 2].element);
    let counts = updated(&page);

    // Then
    assert_eq!(order(&counts), SPLICED);
}

#[test]
fn inserting_a_row_at_the_top_splices_before_the_first() {
    // Given
    let page = page(8);

    // When
    let row = inserted(&page, Some(0), "");
    let counts = updated(&page);

    // Then: the new row paints first and is hit at the top
    assert_eq!(order(&counts), SPLICED);
    let hit = page
        .doc
        .borrow()
        .painted()
        .expect("painted")
        .hit([ROW_X, 2.0]);
    assert_eq!(hit, Some(row));
    matches_full(&page, "insert at the top");
}

#[test]
fn appending_a_row_splices_after_the_last() {
    // Given
    let page = page(8);

    // When
    inserted(&page, None, "");
    let counts = updated(&page);

    // Then
    assert_eq!(order(&counts), SPLICED);
    matches_full(&page, "append");
}

#[test]
fn replacing_a_row_in_one_update_splices_both_runs() {
    // Given
    let page = page(8);

    // When: one row out, another in at its place
    page.doc.borrow_mut().remove(page.rows[3].element);
    inserted(&page, Some(4), "");
    let counts = updated(&page);

    // Then
    assert_eq!(order(&counts), SPLICED);
    matches_full(&page, "replace");
}

#[test]
fn emptying_and_refilling_a_list_splices() {
    // Given
    let page = page(3);

    // When: every row out, two new ones in
    for row in &page.rows {
        page.doc.borrow_mut().remove(row.element);
    }
    inserted(&page, None, "");
    inserted(&page, None, "");
    let counts = updated(&page);

    // Then
    assert_eq!(order(&counts), SPLICED);
    matches_full(&page, "refill");
}

#[test]
fn removing_the_last_row_splices() {
    // Given
    let page = page(8);

    // When
    page.doc.borrow_mut().remove(page.rows[7].element);
    let counts = updated(&page);

    // Then
    assert_eq!(order(&counts), SPLICED);
    matches_full(&page, "remove last");
}
