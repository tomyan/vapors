//! Lanes and rounded clips (flat-scene F3.2b, `docs/plans/frame-clips.md`
//! slices 2 and 3): a lane whose own box rounds its clip frames its rows,
//! an insert costing what it does in a square one; a framed lane a
//! rounded clip from above reaches is cut by it, also when it comes to be framed, its rows hit by the
//! curve. What such a frame refuses is in `rounded_clip_frames.rs`.

use crate::support;

use support::element;
use support::lane_page::header_list;
use support::narrow_list::ROWS;
use support::rounded_page::{at, card_list};
use vapors_dom::{Dom, NodeId};
use vapors_elements::Document;

/// The counts of one row put in at the middle of `count` rows in a list
/// styled `list`.
fn insert_counts(count: usize, list: &str) -> vapors_elements::UpdateCounts {
    let css = format!("body {{ margin: 0; }} .head {{ height: 10px; }} {ROWS} .list {{ {list} }}");
    let page = header_list(count, &css, at(1.0));
    page.edited(at(1.0), |page| _ = page.insert_at(count / 2))
}

#[test]
fn a_row_put_in_a_rounded_list_costs_what_it_does_in_a_square_one() {
    for count in [100, 1000] {
        // When
        let square = insert_counts(count, "overflow: hidden;");
        let rounded = insert_counts(count, "overflow: hidden; border-radius: 8px;");

        // Then: framed, the same boxes moved and painted (F3.2c measured
        // 306 and 3006 painted while a rounded list declined to frame)
        assert_eq!(rounded.lanes_unframed, 0, "{rounded:?}");
        let costs = |c: &vapors_elements::UpdateCounts| (c.boxes_moved, c.boxes_painted);
        assert_eq!(costs(&rounded), costs(&square), "{count} rows");
    }
}

#[test]
fn a_rounded_list_past_the_framing_depth_is_framed_not_refused() {
    // Given: 4000 rows of 20 px, 80,000 px, past the 65,536 px at which a
    // lane frames whatever its children, its rows' cells framed too
    let css = format!(
        "body {{ margin: 0; }} .head {{ height: 10px; }} {ROWS} \
         .list {{ overflow: hidden; border-radius: 8px; }}"
    );

    // When: laid out (an update the page makes, `header_list`)
    let page = header_list(4000, &css, at(1.0));

    // Then
    assert!(
        page.doc.borrow().painted().is_some(),
        "updated, not refused"
    );
}

/// Asserts a point outside the card's top-left curve, inside its
/// rectangle, hits the card, not the first row, and one inside the curve
/// the row: the rows' frame is cut by the card's rounded clip.
fn assert_rows_cut_round(doc: &Document, card: NodeId, row: NodeId) {
    let painted = doc.painted().expect("painted");
    assert_eq!(painted.hit([2.0, 2.0]), Some(card), "outside the curve");
    assert_eq!(painted.hit([6.0, 6.0]), Some(row), "inside the curve");
}

#[test]
fn a_framed_lane_under_a_rounded_clip_from_above_is_cut_by_it() {
    // Given: whole-pixel rows, so the list frames them
    let (mut doc, card, rows) = card_list("");

    // When
    doc.update(at(1.0)).expect("updates");

    // Then
    assert_eq!(doc.update_counts().lanes_framed, 1, "the rows are framed");
    assert_rows_cut_round(&doc, card, rows[0]);
}

#[test]
fn a_lane_under_a_rounded_clip_that_comes_to_be_framed_is_cut_by_it() {
    // Given: a fractional row, so the list declines to frame
    let (mut doc, card, rows) = card_list(".odd { height: 10.5px; }");
    doc.set_attribute(rows[10], "class", "row odd");
    doc.update(at(1.0)).expect("updates");

    // When: the row becomes whole, and the list frames its rows
    doc.set_attribute(rows[10], "class", "row");
    doc.update(at(1.0)).expect("updates");

    // Then
    assert_eq!(doc.update_counts().lanes_framed, 1, "the rows are framed");
    assert_rows_cut_round(&doc, card, rows[0]);
}

#[test]
fn a_clipped_box_in_a_rounded_lane_row_counts_the_rounded_link() {
    // Given: a box under a clipping box in the first row
    let (mut doc, _, rows) = card_list(".cut { overflow: hidden; height: 5px; }");
    let cut = element(&mut doc, rows[0], "div", "cut");
    element(&mut doc, cut, "div", "inside");

    // When
    let painted = doc.update(at(1.0)).expect("updates");

    // Then: its own clip, the clipping box's, then the row's frame: the
    // painter's rounded
    // entry under its square one, two links, so three, as the painter's
    // chain (docs/plans/frame-clips.md, slice 2)
    assert_eq!(painted.longest_clip_chain(), 3);
}
