//! Lanes and rounded clips (flat-scene F3.2b): a lane whose own box
//! rounds its clip declines to frame its rows; a framed lane a rounded
//! clip from above reaches is refused, also when it comes to be framed.

use crate::support;

use support::lane_page::header_list;
use support::narrow_list::ROWS;
use support::rounded_page::{CARD, at};
use support::{document, element, refusals};
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

/// A rounded card holding a list of 40 rows, a lane taller than the card,
/// so its content clip is the card's rounded one; `css` added. The card,
/// and the rows.
fn card_list(css: &str) -> (Document, NodeId, Vec<NodeId>) {
    let mut doc = document();
    let body = doc.body();
    let card = element(&mut doc, body, "div", "card");
    let list = element(&mut doc, card, "div", "list");
    let rows: Vec<NodeId> = (0..40)
        .map(|_| element(&mut doc, list, "div", "row"))
        .collect();
    let css = format!("{CARD} .list {{ height: 400px; }} .row {{ height: 10px; }} {css}");
    doc.add_stylesheet("page.css", &css).expect("parses");
    (doc, card, rows)
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
    assert_rows_cut_round(&doc, card, rows[0]);
}

#[test]
fn a_scroller_in_a_rounded_frame_clear_of_its_corners_is_accepted() {
    // Given: the third row holds a scroller inset from the card's sides
    let (mut doc, _, rows) = card_list(".pane { overflow: auto; height: 10px; margin: 0 20px; }");
    element(&mut doc, rows[2], "div", "pane");

    // Then: the update is not refused
    doc.update(at(1.0)).expect("updates");
}

#[test]
fn a_scroller_in_a_rounded_frame_crossing_its_corners_is_refused() {
    // Given: the first row holds a scroller as wide as the card, across
    // its top corners
    let (mut doc, _, rows) = card_list(".pane { overflow: auto; height: 10px; }");
    element(&mut doc, rows[0], "div", "pane");

    // When
    let errors = refusals(&mut doc);

    // Then: named, with the box that rounds the frame
    let named = |e: &&String| {
        e.contains("inside a frame with a rounded clip") && e.contains("class=\"card\"")
    };
    assert!(errors.iter().any(|e| named(&e)), "{errors:?}");
}

#[test]
fn an_img_in_a_rounded_frame_is_refused() {
    // Given: an image in the third row
    let (mut doc, _, rows) = card_list("img { display: block; }");
    let image = vapors_scene::RgbaImage {
        size: vapors_scene::Size {
            width: 2,
            height: 2,
        },
        rgba: [0, 0, 0, 255].repeat(4),
    };
    doc.register_image("a.png", image).expect("registers");
    let img = element(&mut doc, rows[2], "img", "");
    doc.set_attribute(img, "src", "a.png");

    // When
    let errors = refusals(&mut doc);

    // Then
    assert!(
        errors
            .iter()
            .any(|e| e.contains("is an img under a rounded clip")),
        "{errors:?}"
    );
}
