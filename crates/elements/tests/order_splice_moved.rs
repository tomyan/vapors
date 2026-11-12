//! A box moved out of a removed row keeps its slot, so its steps are
//! already in the kept paint order: the splice declines and the whole
//! tree is walked (`docs/plans/splice.md`, section (c)), rather than
//! inserting the steps a second time. Each also matches a full update
//! through the oracle.

mod support;

use support::rows_page::RowsPage;
use support::splice_page::{assert_declined, inserted, matches_full, page, updated};
use vapors_dom::{Dom, NodeId};
use vapors_elements::SpliceDecline;

/// A row holding a row, both kept by an update of their own.
fn wrapper_with_inner(page: &RowsPage) -> (NodeId, NodeId) {
    let wrapper = inserted(page, Some(4), "");
    let inner = {
        let mut doc = page.doc.borrow_mut();
        let inner = doc.create_element("div");
        doc.set_attribute(inner, "class", "row");
        doc.insert_before(wrapper, inner, None);
        inner
    };
    updated(page);
    (wrapper, inner)
}

#[test]
fn a_child_of_a_removed_row_moved_into_the_owner_is_declined_and_walked() {
    // Given
    let page = page(8);
    let (wrapper, inner) = wrapper_with_inner(&page);

    // When: the inner row moves under the body and the wrapper goes
    {
        let mut doc = page.doc.borrow_mut();
        let body = doc.body();
        doc.insert_before(body, inner, Some(page.rows[6].element));
        doc.remove(wrapper);
    }
    let counts = updated(&page);

    // Then
    assert_declined(counts, SpliceDecline::InsertedHasPlace);
    matches_full(&page, "unwrap");
}

#[test]
fn a_child_of_a_removed_row_moved_into_a_new_row_is_declined_and_walked() {
    // Given
    let page = page(8);
    let (wrapper, inner) = wrapper_with_inner(&page);

    // When: the inner row moves into a new row and the wrapper goes
    let fresh = inserted(&page, Some(6), "");
    {
        let mut doc = page.doc.borrow_mut();
        doc.insert_before(fresh, inner, None);
        doc.remove(wrapper);
    }
    let counts = updated(&page);

    // Then
    assert_declined(counts, SpliceDecline::InsertedHasPlace);
    matches_full(&page, "into new row");
}
