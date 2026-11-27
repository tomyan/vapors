//! Lanes and rounded clips (flat-scene F3.2b, `docs/plans/frame-clips.md`
//! slice 2): a lane whose own box rounds its clip declines to frame its
//! rows (until slice 3); a framed lane a rounded clip from above reaches
//! is cut by it, also when it comes to be framed, its rows hit by the
//! curve. What such a frame refuses is in `rounded_clip_frames.rs`.

use crate::support;

use support::element;
use support::lane_page::header_list;
use support::narrow_list::ROWS;
use support::rounded_page::{at, card_list};
use vapors_dom::{Dom, NodeId};
use vapors_elements::{Document, LaneDecline};

#[test]
fn a_lane_whose_own_box_clips_to_rounded_corners_is_not_framed() {
    // Given: 100 rows in a list that clips them to rounded corners
    let css = format!(
        "body {{ margin: 0; }} .head {{ height: 10px; }} {ROWS} .list {{ overflow: hidden; border-radius: 8px; }}"
    );
    let page = header_list(100, &css, at(1.0));

    // When
    let counts = page.edited(at(1.0), |page| _ = page.insert_at(50));

    // Then
    assert_eq!(
        counts.lane_unframed,
        Some(LaneDecline::LaneFrameRoundedClip),
        "{counts:?}"
    );
}

#[test]
fn a_lane_whose_border_is_wider_than_its_radius_is_framed() {
    // Given: the list's 6px border leaves its 4px corners square inside
    let css = format!(
        "body {{ margin: 0; }} .head {{ height: 10px; }} {ROWS} \
         .list {{ overflow: hidden; border: 6px solid black; border-radius: 4px; }}"
    );
    let page = header_list(100, &css, at(1.0));

    // When
    let counts = page.edited(at(1.0), |page| _ = page.insert_at(50));

    // Then
    assert_eq!(counts.lanes_unframed, 0, "{counts:?}");
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
