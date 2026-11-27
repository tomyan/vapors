use taffy::Style;
use vapors_dom::NodeId;

use super::{Met, holds, meet, padding_radii};
use crate::clip::BoxClip;
use crate::layout::{BoxRect, Content, LayoutBox};

fn rect(x: f32, y: f32, width: f32, height: f32) -> BoxRect {
    BoxRect {
        x,
        y,
        width,
        height,
    }
}

fn bordered(width: f32) -> LayoutBox {
    let mut layout_box = LayoutBox::new(Content::Element(NodeId(1)), Style::default());
    layout_box.layout.border = taffy::Rect {
        left: width,
        right: width,
        top: width,
        bottom: width,
    };
    layout_box
}

/// The card: a 100 x 60 padding box at the origin, 12px corners, slot 3.
const CARD: usize = 3;

fn card() -> BoxClip {
    BoxClip::rounded(rect(0.0, 0.0, 100.0, 60.0), CARD)
}

fn card_radii(slot: usize) -> [f32; 4] {
    assert_eq!(slot, CARD, "only the card is rounded here");
    [12.0; 4]
}

#[test]
fn the_padding_radii_are_the_border_radii_less_the_border() {
    // When
    let radii = padding_radii(
        &bordered(1.0),
        rect(0.0, 0.0, 100.0, 60.0),
        [12.0, 12.0, 0.0, 4.0],
    );

    // Then
    assert_eq!(radii, [11.0, 11.0, 0.0, 3.0]);
}

#[test]
fn a_border_wider_than_its_radius_gives_a_square_clip_corner() {
    // When
    let radii = padding_radii(&bordered(6.0), rect(0.0, 0.0, 100.0, 60.0), [4.0; 4]);

    // Then
    assert_eq!(radii, [0.0; 4]);
}

#[test]
fn radii_past_a_side_are_scaled_down_as_css_says() {
    // Given: two 40px corners on a 40px tall side: each halved
    let radii = padding_radii(&bordered(0.0), rect(0.0, 0.0, 200.0, 40.0), [40.0; 4]);

    // Then
    assert_eq!(radii, [20.0; 4]);
}

#[test]
fn a_rectangle_clear_of_the_corners_is_held_and_one_in_a_corner_is_not() {
    // Then: a row inside the card's straight part is held
    assert!(holds(
        rect(0.0, 0.0, 100.0, 60.0),
        [12.0; 4],
        rect(0.0, 12.0, 100.0, 20.0)
    ));
    // a row across the top corners is not
    assert!(!holds(
        rect(0.0, 0.0, 100.0, 60.0),
        [12.0; 4],
        rect(0.0, 0.0, 100.0, 20.0)
    ));
    // the same row in a square box is
    assert!(holds(
        rect(0.0, 0.0, 100.0, 60.0),
        [0.0; 4],
        rect(0.0, 0.0, 100.0, 20.0)
    ));
    // nor is one past the rectangle
    assert!(!holds(
        rect(0.0, 0.0, 100.0, 60.0),
        [12.0; 4],
        rect(0.0, 20.0, 101.0, 20.0)
    ));
}

#[test]
fn a_corner_point_on_the_curve_is_held() {
    // Given: the point on the top-left circle at 45 degrees
    let at = 12.0 - 12.0 * std::f32::consts::FRAC_1_SQRT_2 + 0.001;

    // Then
    assert!(holds(
        rect(0.0, 0.0, 100.0, 60.0),
        [12.0; 4],
        rect(at, at, 10.0, 10.0)
    ));
    assert!(!holds(
        rect(0.0, 0.0, 100.0, 60.0),
        [12.0; 4],
        rect(1.0, 1.0, 10.0, 10.0)
    ));
}

#[test]
fn a_rounded_box_under_a_larger_square_clip_keeps_its_corners() {
    // When
    let met = meet(
        Some(BoxClip::square(rect(-10.0, -10.0, 200.0, 200.0))),
        card(),
        &card_radii,
    );

    // Then
    assert_eq!(met, Met::Clip(card()));
}

#[test]
fn a_square_box_inside_a_rounded_clip_clear_of_its_corners_is_square() {
    // Given
    let row = BoxClip::square(rect(0.0, 20.0, 100.0, 20.0));

    // When
    let met = meet(Some(card()), row, &card_radii);

    // Then
    assert_eq!(met, Met::Clip(row));
}

#[test]
fn a_square_box_wider_than_a_rounded_clip_takes_the_rounded_clip() {
    // When
    let met = meet(
        Some(card()),
        BoxClip::square(rect(-5.0, -5.0, 200.0, 200.0)),
        &card_radii,
    );

    // Then
    assert_eq!(met, Met::Clip(card()));
}

#[test]
fn a_square_clip_across_a_rounded_corner_is_an_overlap() {
    // Given: a clipping row over the card's top corners
    let row = BoxClip::square(rect(0.0, 0.0, 100.0, 20.0));

    // When
    let met = meet(Some(card()), row, &card_radii);

    // Then
    assert_eq!(met, Met::Overlap(row));
}

#[test]
fn a_rounded_box_cut_by_a_square_clip_is_an_overlap() {
    // When: a square clip ending halfway down the card
    let met = meet(
        Some(BoxClip::square(rect(0.0, 0.0, 100.0, 30.0))),
        card(),
        &card_radii,
    );

    // Then
    assert_eq!(
        met,
        Met::Overlap(BoxClip::square(rect(0.0, 0.0, 100.0, 30.0)))
    );
}

#[test]
fn a_rounded_box_wholly_outside_a_clip_is_cut_to_nothing() {
    // When
    let met = meet(
        Some(BoxClip::square(rect(0.0, 100.0, 100.0, 30.0))),
        card(),
        &card_radii,
    );

    // Then
    assert!(
        matches!(met, Met::Clip(clip) if clip.rounded_by().is_none() && clip.rect.height == 0.0)
    );
}

#[test]
fn two_square_clips_meet_at_their_intersection() {
    // When
    let a = BoxClip::square(rect(0.0, 0.0, 100.0, 30.0));
    let met = meet(
        Some(a),
        BoxClip::square(rect(50.0, 10.0, 100.0, 30.0)),
        &card_radii,
    );

    // Then
    assert_eq!(
        met,
        Met::Clip(BoxClip::square(rect(50.0, 10.0, 50.0, 20.0)))
    );
}
