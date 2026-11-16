//! An insert or removal under a lane's container rebuilds the container
//! as an owner whose children are edited in place (`child_edits.rs`): its
//! children list differs from its box's before, so it is changed, and
//! only the inserted children's subtrees are lent the taken caches
//! (4h, `docs/plans/splice-layout.md` section 12). Lending visits the
//! same boxes on a list of 100 rows and of 400, and the layout and
//! placement match a full update (`assert_matches_full`; in debug builds
//! the rebuild and layout checks run on every update too).

use crate::support;

use support::lane_page::{HeaderList, header_list};
use support::narrow_list::{ROWS, VIEW};
use vapors_dom::Dom;
use vapors_elements::UpdateCounts;

/// `count` rows under a header, laid out once.
fn list(count: usize) -> HeaderList {
    header_list(count, &format!("body {{ margin: 0; }} {ROWS}"), VIEW())
}

/// The counts of putting a row in before row 5, matched against a full
/// update.
fn inserted(page: &HeaderList) -> UpdateCounts {
    page.edited(VIEW(), |page| {
        page.insert_at(5);
    })
}

#[test]
fn an_insert_lends_to_the_same_boxes_on_a_list_four_times_the_size() {
    // Given
    let (small, large) = (list(100), list(400));

    // When
    let (a, b) = (inserted(&small), inserted(&large));

    // Then: the inserted row's boxes, not every row
    assert_eq!(a.boxes_lend_visited, b.boxes_lend_visited);
    let visited = a.boxes_lend_visited;
    assert!(visited > 0 && visited < 10, "{visited}");
}

#[test]
fn an_insert_and_a_removal_in_one_owner_match_a_full_update() {
    // Given
    let page = list(100);

    // When: a row put in before row 5 and row 40 removed, in one update
    let counts = page.edited(VIEW(), |page| {
        page.insert_at(5);
        page.doc.borrow_mut().remove(page.rows[40]);
    });

    // Then: matched against a full update inside `edited`; one owner
    // edited in place
    assert_eq!(counts.children_edited, 1);
}
