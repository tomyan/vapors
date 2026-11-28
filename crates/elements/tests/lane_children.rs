//! An owner whose children are edited in place, as a lane's container is
//! by an insert or a removal, finds where each edit falls through its
//! lane (O(log n), `LaneTree::rank`) and derives the run of children the
//! edits changed from those places (4h): it searches and compares none
//! of its children one by one, on a list of 100 rows or of 400. Every
//! build checks the lane's place holds the child; debug builds check the
//! run against a compare (`slots_edit_places.rs`, `reshaped.rs`). An
//! insert is put after its previous sibling's slot and the children are
//! assembled once (action 49), so a second insert in one update searches
//! nothing. Where the lane cannot answer the changed run's place (an
//! owner without a lane), that search is counted.

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

/// The counts of a row put in before row 5 and another before the
/// fifth row from the end, in one update.
fn inserted_twice(page: &HeaderList) -> UpdateCounts {
    page.edited(VIEW(), |page| {
        page.insert_at(5);
        page.insert_at(page.rows.len() - 5);
    })
}

#[test]
fn a_second_insert_in_one_owner_in_one_update_scans_no_child() {
    // Given
    let (small, large) = (list(100), list(400));

    // When
    let (a, b) = (inserted_twice(&small), inserted_twice(&large));

    // Then: each insert goes after its previous sibling's slot and the
    // children are assembled once, so the second insert's place is not
    // searched for (it was, up to it: 96 and 396 entries, the known term
    // splice-layout.md section 12 named, removed by action 49)
    assert_eq!((a.children_scanned, b.children_scanned), (0, 0));
}

/// The counts of a row put in before row 5 in a grid list, which no
/// lane lays out.
fn inserted_into_grid() -> UpdateCounts {
    let css = format!("body {{ margin: 0; }} {ROWS} .list {{ display: grid; }}");
    header_list(100, &css, VIEW()).edited(VIEW(), |page| {
        page.insert_at(5);
    })
}

#[test]
fn an_insert_into_an_owner_without_a_lane_searches_once_for_the_changed_run() {
    // Given, when
    let counts = inserted_into_grid();

    // Then: row 4, the new row's previous, is searched for once, up to
    // it, for the changed run's place (`reshaped_by`); the new row is put
    // after row 4's slot with no search (before action 49, a second
    // search put it there: 10 entries)
    assert_eq!(counts.children_scanned, 5);
}
