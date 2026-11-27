//! Lanes and rounded clips (flat-scene F3.2b, `docs/plans/frame-clips.md`
//! slices 2 and 3): a lane whose own box rounds its clip frames its rows,
//! an insert costing what it does in a square one; a framed lane a
//! rounded clip from above reaches is cut by it, also when it comes to be framed, its rows hit by the
//! curve. What such a frame refuses is in `rounded_clip_frames.rs`.

use crate::support;

use support::lane_page::header_list;
use support::narrow_list::ROWS;
use support::rounded_page::{at, card_list};
use support::{element, refusals};
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
        assert_eq!(rounded.lanes_framed, 1, "{rounded:?}");
        let costs = |c: &vapors_elements::UpdateCounts| (c.boxes_moved, c.boxes_painted);
        assert_eq!(costs(&rounded), costs(&square), "{count} rows");
    }
}

#[test]
fn a_rounded_list_past_the_framing_depth_updates() {
    // Given: 4000 rows of 20 px, 80,000 px, past the 65,536 px at which a
    // lane frames whatever its children (refused before slice 2, while
    // the list declined to frame and its rows' cells framed for depth)
    let css = format!(
        "body {{ margin: 0; }} .head {{ height: 10px; }} {ROWS} \
         .list {{ overflow: hidden; border-radius: 8px; }}"
    );

    // When: laid out (an update the page makes, `header_list`)
    let page = header_list(4000, &css, at(1.0));

    // Then: updated (`header_list` expects it) with its rows framed
    let doc = page.doc.borrow();
    assert!(
        doc.update_counts().lanes_framed == 1,
        "{:?}",
        doc.update_counts()
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

/// A list of 40 rows whose own box clips to 8 px corners when its class
/// holds `round`; the page.
fn self_rounded_list() -> support::lane_page::HeaderList {
    let css = format!(
        "body {{ margin: 0; }} .head {{ height: 10px; }} {ROWS} \
         .list {{ overflow: hidden; }} .round {{ border-radius: 8px; }}"
    );
    let page = header_list(40, &css, at(1.0));
    page.doc
        .borrow_mut()
        .set_attribute(page.list, "class", "list round");
    page.doc.borrow_mut().update(at(1.0)).expect("updates");
    page
}

#[test]
fn a_list_that_rounds_itself_frames_its_rows_cut_by_its_curve() {
    // Given: the list at y 10, its 8 px top-left curve about (8, 18),
    // laid out in one update
    let css = format!(
        "body {{ margin: 0; }} .head {{ height: 10px; }} {ROWS} \
         .list {{ overflow: hidden; border-radius: 8px; }}"
    );
    let page = header_list(40, &css, at(1.0));
    let doc = page.doc.borrow();

    // Then: framed; outside the curve the list is hit, inside it the first
    // row (its first cell, there)
    assert_eq!(doc.update_counts().lanes_framed, 1, "the rows are framed");
    let painted = doc.painted().expect("painted");
    assert_eq!(
        painted.hit([1.0, 11.0]),
        Some(page.list),
        "outside the curve"
    );
    let cell = doc.children(page.rows[0])[0];
    assert_eq!(painted.hit([5.0, 15.0]), Some(cell), "inside the curve");
}

#[test]
fn an_img_in_a_list_that_rounds_itself_follows_its_radius() {
    // Given: the rounded list, an image in its first row
    let page = self_rounded_list();
    let mut doc = page.doc.borrow_mut();
    let image = vapors_scene::RgbaImage {
        size: vapors_scene::Size {
            width: 2,
            height: 2,
        },
        rgba: [0, 0, 0, 255].repeat(4),
    };
    doc.register_image("a.png", image).expect("registers");
    let img = doc.create_element("img");
    doc.set_attribute(img, "src", "a.png");
    doc.append(page.rows[0], img);

    // Then: refused as an image under a rounded clip while rounded;
    // accepted once square; refused so again
    let image_refused = |doc: &mut Document| {
        let errors = refusals(doc);
        let named = errors
            .iter()
            .any(|e| e.contains("is an img under a rounded clip"));
        assert!(named, "{errors:?}");
    };
    image_refused(&mut doc);
    doc.set_attribute(page.list, "class", "list");
    doc.update(at(1.0)).expect("square: accepted");
    doc.set_attribute(page.list, "class", "list round");
    image_refused(&mut doc);
}

#[test]
fn a_framed_rows_items_on_the_page_are_under_the_frames_rounded_clip() {
    // Given: blue rows in a framed lane under the card's 12 px corners
    let (mut doc, ..) = card_list(".row { background-color: blue; }");

    // When
    doc.update(at(1.0)).expect("updates");

    // Then: the first row's quad, read on the page, is under the card's
    // padding box with its 11 px radii, as the painter chains it
    let clip = support::rounded_page::fill_clip(&doc).expect("a clip");
    assert_eq!(clip.radii(), [11.0; 4]);
}

/// The page clip of the blue quad of `doc`, a row's child `inner`, in a
/// card list styled `css`.
fn inner_clip(css: &str, inner: &str) -> vapors_scene::Clip {
    let css = format!("{css} .blue {{ height: 4px; background-color: blue; }}");
    let (mut doc, _, rows) = card_list(&css);
    let holder = element(&mut doc, rows[2], "div", inner);
    element(&mut doc, holder, "div", "blue");
    doc.update(at(1.0)).expect("updates");
    support::rounded_page::fill_clip(&doc).expect("a clip")
}

#[test]
fn a_square_clip_in_a_rounded_frame_is_the_frames_rounded_clip_cut_to_it() {
    // When: the third row, y 21..31, holds a clipping box x 31..71
    let clip = inner_clip(
        ".cut { overflow: hidden; height: 5px; margin: 0 30px; }",
        "cut",
    );

    // Then: the card's rounded clip, cut to the box on the page
    assert_eq!(clip.radii(), [11.0; 4]);
    assert_eq!(
        clip.bounds(),
        vapors_scene::Rect::new(31.0, 21.0, 71.0, 26.0)
    );
}

#[test]
fn a_rounded_clip_held_clear_of_a_rounded_frames_corners_is_its_own() {
    // When: the third row holds a box with 2 px corners, inset 30 px
    let css = ".bubble { overflow: hidden; height: 5px; margin: 0 30px; border-radius: 2px; }";
    let clip = inner_clip(css, "bubble");

    // Then: on the page, its own corners, which the card's curve holds
    assert_eq!(clip.radii(), [2.0; 4]);
    assert_eq!(clip.rect(), vapors_scene::Rect::new(31.0, 21.0, 71.0, 26.0));
}
