//! A shape change the order splice cannot decide walks the whole tree
//! and counts `splices_declined` (`docs/plans/splice.md`, section (c)):
//! a positioned row, a scrolling row, a z-index in a row, an owner
//! whose kept children were reordered, a positioned row removed, a kept
//! row restacked, and a kept row moved into a new row. Each asserts the
//! condition that declined (`SpliceDecline`) and also matches a full
//! update through the oracle.

mod support;

use support::splice_page::{assert_declined, inserted, matches_full, page, updated};
use vapors_dom::Dom;
use vapors_elements::SpliceDecline;

#[test]
fn a_positioned_row_is_declined_and_walked() {
    // Given
    let page = page(8);

    // When
    inserted(&page, Some(4), "positioned");
    let counts = updated(&page);

    // Then
    assert_declined(counts, SpliceDecline::InsertedStacked);
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
    assert_declined(counts, SpliceDecline::InsertedStacked);
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
    assert_declined(counts, SpliceDecline::InsertedStacked);
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
    assert_declined(counts, SpliceDecline::Reordered);
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
    assert_declined(counts, SpliceDecline::FreedStacked);
    matches_full(&page, "remove positioned");
}

#[test]
fn a_kept_row_restacked_by_the_insert_is_declined_and_walked() {
    // Given: the second row is positioned
    let page = page(8);
    page.doc
        .borrow_mut()
        .add_stylesheet("nth.css", ".row:nth-child(2) { position: relative; }")
        .expect("parses");
    updated(&page);

    // When: a row at the top makes the old second row the third
    inserted(&page, Some(0), "");
    let counts = updated(&page);

    // Then
    assert_declined(counts, SpliceDecline::Restacked);
    matches_full(&page, "restack");
}

#[test]
fn a_kept_row_moved_into_a_new_row_is_declined_and_walked() {
    // Given
    let page = page(8);

    // When
    let wrapper = inserted(&page, Some(4), "");
    page.doc
        .borrow_mut()
        .insert_before(wrapper, page.rows[3].element, None);
    let counts = updated(&page);

    // Then
    assert_declined(counts, SpliceDecline::Moved);
    matches_full(&page, "wrap");
}
