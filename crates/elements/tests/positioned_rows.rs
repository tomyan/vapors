//! The positioned rows page (`support/positioned_rows.rs`, flat-scene
//! W6): a class edit on a positioned row and an insert before the
//! middle row each give what a full update gives; a positioned row put
//! in or taken out declines the splice for its stacking; and the page's
//! out-of-flow boxes are what declines the body's lane.

use crate::support;

use support::oracle::assert_matches_full;
use support::positioned_rows::{
    insert_positioned, make_positioned, position_rows, toggle_positioned_marked,
};
use support::rows_page::{RowsPage, row_before, rows_page, rows_viewport};
use vapors_dom::Dom;
use vapors_elements::{LaneDecline, SpliceDecline};

#[test]
fn a_class_edit_on_a_positioned_row_matches_a_full_update() {
    // Given
    let page = rows_page(100);
    make_positioned(&page);

    // When, then: added, and removed
    for case in ["added", "removed"] {
        toggle_positioned_marked(&page);
        page.doc
            .borrow_mut()
            .update(rows_viewport())
            .expect("updates");
        assert_matches_full(&page.doc, rows_viewport(), case);
    }
}

#[test]
fn an_insert_among_positioned_rows_matches_a_full_update() {
    // Given
    let page = rows_page(100);
    make_positioned(&page);

    // When
    let middle = page.rows[50].element;
    let added = row_before(&mut page.doc.borrow_mut(), 100, Some(middle)).element;
    page.doc
        .borrow_mut()
        .update(rows_viewport())
        .expect("updates");

    // Then: inserted, and removed again
    assert_matches_full(&page.doc, rows_viewport(), "inserted");
    page.doc.borrow_mut().remove(added);
    page.doc
        .borrow_mut()
        .update(rows_viewport())
        .expect("updates");
    assert_matches_full(&page.doc, rows_viewport(), "removed");
}

#[test]
fn a_positioned_row_put_in_and_taken_out_declines_the_splice_for_its_stacking() {
    // Given
    let page = rows_page(100);
    make_positioned(&page);

    // When: put in, then taken out
    let added = insert_positioned(&page, 100);
    page.doc
        .borrow_mut()
        .update(rows_viewport())
        .expect("updates");
    let inserted = page.doc.borrow().update_counts().splice_decline;
    page.doc.borrow_mut().remove(added);
    page.doc
        .borrow_mut()
        .update(rows_viewport())
        .expect("updates");
    let removed = page.doc.borrow().update_counts().splice_decline;

    // Then
    assert_eq!(
        (inserted, removed),
        (
            Some(SpliceDecline::InsertedStacked),
            Some(SpliceDecline::FreedStacked)
        )
    );
    assert_matches_full(&page.doc, rows_viewport(), "removed");
}

/// Whether the body's lane frames its rows after an insert on the page
/// built by `make`, and the declines counted.
fn body_lane_after_an_insert(make: fn(&RowsPage)) -> (usize, usize) {
    let page = rows_page(100);
    make(&page);
    let middle = page.rows[50].element;
    row_before(&mut page.doc.borrow_mut(), 100, Some(middle));
    page.doc
        .borrow_mut()
        .update(rows_viewport())
        .expect("updates");
    let counts = page.doc.borrow().update_counts();
    (counts.lanes_framed, counts.lanes_declined)
}

#[test]
fn the_out_of_flow_boxes_cost_the_body_its_lane() {
    // Given, when: the page with its rows positioned, and with its
    // absolute box and fixed footer too
    let in_flow = body_lane_after_an_insert(position_rows);
    let whole = body_lane_after_an_insert(make_positioned);

    // Then: the body's lane frames its rows without them, and is
    // declined with them, one decline more (OutOfFlowChild: the test
    // below). splice-layout slice 3b takes them into the lane; this test
    // then fails, and its pull request judges `--scaling
    // PositionedInsert` as Insert (crates/perf/src/scaling_judged.rs).
    assert_eq!((in_flow.0, whole.0), (1, 0), "lanes framed");
    assert_eq!(whole.1, in_flow.1 + 1, "lanes declined");
}

#[test]
fn a_fixed_child_of_the_body_declines_its_lane_as_out_of_flow() {
    // Given: block rows under the body, and the page's fixed footer
    let mut doc = support::document();
    let body = doc.body();
    for _ in 0..40 {
        support::element(&mut doc, body, "div", "row");
    }
    support::element(&mut doc, body, "div", "footer");
    let css = "body { margin: 0; } .row { height: 24px; } \
        .footer { position: fixed; left: 0; bottom: 0; width: 640px; height: 24px; }";
    doc.add_stylesheet("p.css", css).expect("parses");

    // When
    doc.update(rows_viewport()).expect("updates");

    // Then
    assert_eq!(
        doc.update_counts().lane_decline,
        Some(LaneDecline::OutOfFlowChild)
    );
}
