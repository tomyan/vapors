//! A splice followed by patching (`docs/plans/paint-order-labels.md`,
//! slice 2; `paint_partial_drawn.rs` `many`): on a small page the
//! drawing steps, the extents index and the hits follow the splice's
//! edits, counted in `order_splices_patched`, for a row put in, a row
//! taken out and both in one update; in a framed list, inserts that
//! relabel a range are patched too; a splice that moves most rows of a
//! long unframed list makes the three afresh. The debug build compares each of the three with one made
//! afresh after every update (`kept_paint_check.rs`), and each case
//! matches a full update.

mod support;

use support::lane_page::{assert_drawn_afresh, header_list};
use support::narrow_list::{ROWS, VIEW};
use support::oracle::assert_matches_full;
use support::splice_page::{SPLICED, inserted, matches_full, order, page, updated};
use vapors_dom::Dom;
use vapors_elements::UpdateCounts;

/// The counts show a splice, patched.
fn assert_patched(counts: &UpdateCounts) {
    assert_eq!(order(counts), SPLICED, "{counts:?}");
    assert_eq!(counts.order_splices_patched, 1, "{counts:?}");
}

#[test]
fn a_row_put_in_a_small_page_is_patched() {
    // Given
    let page = page(8);

    // When
    inserted(&page, Some(4), "");
    let counts = updated(&page);

    // Then
    assert_patched(&counts);
    matches_full(&page, "insert");
}

#[test]
fn a_row_taken_out_of_a_small_page_is_patched() {
    // Given
    let page = page(8);

    // When
    page.doc.borrow_mut().remove(page.rows[4].element);
    let counts = updated(&page);

    // Then
    assert_patched(&counts);
    matches_full(&page, "remove");
}

#[test]
fn a_row_taken_out_and_one_put_in_in_one_update_are_patched() {
    // Given
    let page = page(8);

    // When
    page.doc.borrow_mut().remove(page.rows[2].element);
    inserted(&page, Some(5), "");
    let counts = updated(&page);

    // Then
    assert_patched(&counts);
    matches_full(&page, "remove and insert");
}

#[test]
fn rows_put_in_at_two_places_in_turn_in_a_framed_list_relabel_a_range_while_patched() {
    // Given: a framed list of 200 rows, so an insert moves no row
    let page = header_list(200, &format!("{ROWS} body {{ margin: 0; }}"), VIEW());

    // When: rows put in before the sixth and the sixteenth in turn until
    // a gap runs out
    let mut relabelled = 0;
    for turn in 0..150 {
        let at = if turn % 2 == 0 { 5 } else { 15 };
        page.insert_at(at);
        let counts = {
            let mut doc = page.doc.borrow_mut();
            doc.update(VIEW()).expect("updates");
            doc.update_counts()
        };
        assert_patched(&counts);
        relabelled += counts.paint_relabelled;
    }

    // Then
    assert!(relabelled > 0, "no gap ran out in 150 inserts");
    assert_matches_full(&page.doc, VIEW(), "after the inserts");
    assert_drawn_afresh(&page.doc, VIEW(), "after the inserts");
}

#[test]
fn a_row_put_in_near_the_top_of_a_long_unframed_list_is_made_afresh() {
    // Given: 400 rows under body, not a framed lane, so every later row
    // moves
    let page = page(400);

    // When
    inserted(&page, Some(10), "");
    let counts = updated(&page);

    // Then: spliced, and the three made afresh
    assert_eq!(order(&counts), SPLICED, "{counts:?}");
    assert_eq!(counts.order_splices_patched, 0, "{counts:?}");
    assert_eq!(counts.lanes_framed, 0, "{counts:?}");
    matches_full(&page, "insert near the top");
}
