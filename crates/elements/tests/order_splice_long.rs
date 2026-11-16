//! The order splice on an owner with more children than one compared
//! chunk (`order_splice_children.rs`): the runs put in and the kept
//! children are found between the head and tail the old and new
//! children share, so inserts far into the list, two at once, a removal,
//! a reorder and a kept row moved into a new row each give what a full
//! update gives; so does a row appended into a freed slot, which the
//! clip tree puts among its parent's dependents in slot order.

mod support;

use support::splice_page::{
    SPLICED, assert_declined, inserted, matches_full, order, page, updated,
};
use vapors_dom::Dom;
use vapors_elements::SpliceDecline;

/// Rows past two chunks of 64.
const ROWS: usize = 200;

#[test]
fn a_row_inserted_past_two_chunks_is_spliced() {
    // Given
    let page = page(ROWS);

    // When
    inserted(&page, Some(150), "");
    let counts = updated(&page);

    // Then
    assert_eq!(order(&counts), SPLICED);
    matches_full(&page, "insert at 150");
}

#[test]
fn rows_inserted_near_both_ends_at_once_are_spliced() {
    // Given
    let page = page(ROWS);

    // When
    inserted(&page, Some(10), "");
    inserted(&page, Some(180), "");
    let counts = updated(&page);

    // Then
    assert_eq!(order(&counts), SPLICED);
    matches_full(&page, "inserts at 10 and 180");
}

#[test]
fn a_row_removed_past_one_chunk_is_spliced() {
    // Given
    let page = page(ROWS);

    // When
    page.doc.borrow_mut().remove(page.rows[100].element);
    let counts = updated(&page);

    // Then
    assert_eq!(order(&counts), SPLICED);
    matches_full(&page, "remove at 100");
}

#[test]
fn kept_children_reordered_past_two_chunks_are_declined() {
    // Given
    let page = page(ROWS);

    // When: a kept row moved before another kept row
    {
        let mut doc = page.doc.borrow_mut();
        let body = doc.body();
        doc.insert_before(body, page.rows[150].element, Some(page.rows[130].element));
    }
    let counts = updated(&page);

    // Then
    assert_declined(&counts, SpliceDecline::Reordered);
    matches_full(&page, "reorder at 130");
}

#[test]
fn a_kept_row_moved_into_a_new_row_past_two_chunks_is_declined() {
    // Given
    let page = page(ROWS);

    // When
    let wrapper = inserted(&page, Some(150), "");
    page.doc
        .borrow_mut()
        .insert_before(wrapper, page.rows[140].element, None);
    let counts = updated(&page);

    // Then
    assert_declined(&counts, SpliceDecline::Moved);
    matches_full(&page, "wrap at 140");
}

#[test]
fn a_row_appended_into_a_freed_slot_keeps_the_clip_tree_in_slot_order() {
    // Given: a row removed, its slots freed for the next new keys
    let page = page(ROWS);
    page.doc.borrow_mut().remove(page.rows[2].element);
    updated(&page);

    // When: a row appended takes the lowest freed slots, below every
    // kept sibling's
    inserted(&page, None, "");
    let counts = updated(&page);

    // Then: spliced, and the order check found the clip tree as walked
    assert_eq!(order(&counts), SPLICED);
    matches_full(&page, "append into a freed slot");
}
