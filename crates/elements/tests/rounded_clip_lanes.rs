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
/// so its content clip is the card's rounded one; `odd` sets the 11th
/// row's height.
fn card_list(odd: &str) -> (Document, NodeId) {
    let mut doc = document();
    let body = doc.body();
    let card = element(&mut doc, body, "div", "card");
    let list = element(&mut doc, card, "div", "list");
    let rows: Vec<NodeId> = (0..40)
        .map(|_| element(&mut doc, list, "div", "row"))
        .collect();
    doc.set_attribute(rows[10], "class", "row odd");
    let css = format!(
        "{CARD} .list {{ height: 400px; }} .row {{ height: 10px; }} .odd {{ height: {odd}; }}"
    );
    doc.add_stylesheet("page.css", &css).expect("parses");
    (doc, rows[10])
}

#[test]
fn a_framed_lane_under_a_rounded_clip_from_above_is_refused() {
    // Given: whole-pixel rows, so the list frames them
    let (mut doc, _) = card_list("10px");

    // When
    let errors = refusals(&mut doc);

    // Then
    assert!(
        errors.iter().any(|e| e.contains("frames a lane's rows")),
        "{errors:?}"
    );
}

#[test]
fn a_lane_under_a_rounded_clip_that_comes_to_be_framed_is_refused() {
    // Given: a fractional row, so the list declines to frame
    let (mut doc, odd) = card_list("10.5px");
    doc.update(at(1.0)).expect("updates");

    // When: the row becomes whole, and the list frames its rows
    doc.set_attribute(odd, "class", "row");
    let errors = refusals(&mut doc);

    // Then
    assert!(
        errors.iter().any(|e| e.contains("frames a lane's rows")),
        "{errors:?}"
    );
}
