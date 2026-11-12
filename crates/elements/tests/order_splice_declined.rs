//! A shape change the order splice cannot decide walks the whole tree
//! and counts `splices_declined` (`docs/plans/splice.md`, section (c)):
//! a positioned row, a scrolling row, a z-index in a row, an owner
//! whose kept children were reordered, and a positioned row removed.
//! Each also matches a full update through the oracle.

mod support;

use support::splice_page::{DECLINED_WALK, inserted, matches_full, order, page, updated};
use vapors_dom::Dom;

#[test]
fn a_positioned_row_is_declined_and_walked() {
    // Given
    let page = page(8);

    // When
    inserted(&page, Some(4), "positioned");
    let counts = updated(&page);

    // Then
    assert_eq!(order(counts), DECLINED_WALK);
    matches_full(&page, "positioned row");
}

#[test]
fn a_scrolling_row_is_declined_and_walked() {
    // Given
    let page = page(8);

    // When
    inserted(&page, Some(4), "scrolling");
    let counts = updated(&page);

    // Then
    assert_eq!(order(counts), DECLINED_WALK);
    matches_full(&page, "scrolling row");
}

#[test]
fn a_z_index_in_a_row_is_declined_and_walked() {
    // Given
    let page = page(8);

    // When: the z-index is on a cell, below the row's own box
    inserted(&page, Some(4), "raised");
    let counts = updated(&page);

    // Then
    assert_eq!(order(counts), DECLINED_WALK);
    matches_full(&page, "z-index in a row");
}

#[test]
fn reordered_kept_children_are_declined_and_walked() {
    // Given
    let page = page(8);

    // When: a kept row moved before another kept row
    {
        let mut doc = page.doc.borrow_mut();
        let body = doc.body();
        doc.insert_before(body, page.rows[5].element, Some(page.rows[2].element));
    }
    let counts = updated(&page);

    // Then
    assert_eq!(order(counts), DECLINED_WALK);
    matches_full(&page, "reorder");
}

#[test]
fn removing_a_positioned_row_is_declined_and_walked() {
    // Given: a positioned row, kept
    let page = page(8);
    let row = inserted(&page, Some(4), "positioned");
    updated(&page);

    // When
    page.doc.borrow_mut().remove(row);
    let counts = updated(&page);

    // Then: its steps are not in the flow run
    assert_eq!(order(counts), DECLINED_WALK);
    matches_full(&page, "remove positioned");
}
