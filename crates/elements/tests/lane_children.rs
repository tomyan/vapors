//! An owner whose children are edited in place, as a lane's container is
//! by an insert or a removal, finds where each edit falls through its
//! lane (O(log n), `LaneTree::rank`) and derives the run of children the
//! edits changed from those places (4h): it searches and compares none
//! of its children one by one, on a list of 100 rows or of 400. Debug
//! builds check each place against a search and the run against a
//! compare (`slots_edit_places.rs`, `reshaped.rs`).

use crate::support;

use support::lane_page::{HeaderList, header_list};
use support::narrow_list::{ROWS, VIEW};
use vapors_dom::Dom;
use vapors_elements::UpdateCounts;

/// `count` rows under a header, laid out once.
fn list(count: usize) -> HeaderList {
    header_list(count, &format!("body {{ margin: 0; }} {ROWS}"), VIEW())
}

/// The counts of a row put in before row 5 and row 40 removed, in one
/// update, matched against a full update.
fn edited(page: &HeaderList) -> UpdateCounts {
    page.edited(VIEW(), |page| {
        page.insert_at(5);
        page.doc.borrow_mut().remove(page.rows[40]);
    })
}

#[test]
fn an_insert_and_a_removal_scan_no_child_on_a_list_four_times_the_size() {
    // Given
    let (small, large) = (list(100), list(400));

    // When
    let (a, b) = (edited(&small), edited(&large));

    // Then: the lane answered every place
    assert_eq!((a.children_scanned, b.children_scanned), (0, 0));
    assert_eq!((a.children_edited, b.children_edited), (1, 1));
}
