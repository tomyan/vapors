//! Placement chains each slot to the slot it is placed in
//! (`Placement::chain_children`). An owner edited in place, as a lane's
//! container is by an insert, chains only the children it inserted
//! (4h): an insert chains the same few children on a list of 100 rows
//! and of 400. Every other way a slot's chain can change, in the same
//! update as an insert, matches a full update (`assert_matches_full`;
//! debug builds also compare the placement with a fresh one on every
//! update, `placement_check.rs`): a row moved under another row, a row
//! taken out of flow and put back, a row hidden and shown.

use crate::support;

use support::lane_page::{HeaderList, header_list};
use support::narrow_list::{ROWS, VIEW};
use vapors_dom::Dom;

/// `count` rows under a header, laid out once.
fn list(count: usize) -> HeaderList {
    let css = format!("body {{ margin: 0; }} {ROWS} .out {{ position: absolute; top: 3px; }}");
    header_list(count, &css, VIEW())
}

#[test]
fn an_insert_chains_the_same_children_on_a_list_four_times_the_size() {
    // Given
    let (small, large) = (list(100), list(400));

    // When
    let insert = |page: &HeaderList| {
        page.edited(VIEW(), |page| {
            page.insert_at(5);
        })
    };
    let (a, b) = (insert(&small), insert(&large));

    // Then: the inserted row's children, not every row
    assert_eq!(a.slots_chained, b.slots_chained);
    assert!(a.slots_chained > 0, "the inserted row is chained");
}

#[test]
fn a_row_moved_under_another_row_with_an_insert_matches_a_full_update() {
    // Given
    let page = list(100);

    // When: a row put in before row 5, and row 30 moved into row 40, in
    // one update
    page.edited(VIEW(), |page| {
        page.insert_at(5);
        page.doc
            .borrow_mut()
            .insert_before(page.rows[40], page.rows[30], None);
    });

    // Then: matched against a full update inside `edited`
}

#[test]
fn a_row_taken_out_of_flow_and_put_back_with_inserts_matches_a_full_update() {
    // Given
    let page = list(100);

    // When: an insert and row 30 made absolute, then an insert and row 30
    // made static again
    page.edited(VIEW(), |page| {
        page.insert_at(5);
        page.doc
            .borrow_mut()
            .set_attribute(page.rows[30], "class", "row out");
    });
    page.edited(VIEW(), |page| {
        page.insert_at(6);
        page.doc
            .borrow_mut()
            .set_attribute(page.rows[30], "class", "row");
    });

    // Then: each matched against a full update inside `edited`
}

#[test]
fn a_row_hidden_and_shown_with_inserts_matches_a_full_update() {
    // Given
    let page = list(100);

    // When
    page.edited(VIEW(), |page| {
        page.insert_at(5);
        page.doc
            .borrow_mut()
            .set_attribute(page.rows[30], "style", "display: none");
    });
    page.edited(VIEW(), |page| {
        page.insert_at(6);
        page.doc
            .borrow_mut()
            .set_attribute(page.rows[30], "style", "");
    });

    // Then: each matched against a full update inside `edited`
}
