//! Moves `kept_shift.rs` refuses because a snapped edge would not move
//! with the box, and the part entries paint derives from more than the
//! rect.

use vapors_dom::NodeId;

use super::Shift;
use super::tests::{from, kept, painting, shift_of};
use crate::kept_box::KeptBox;
use crate::layout::{BoxRect, Content};

fn tall(y: f32, height: f32) -> BoxRect {
    BoxRect {
        x: 0.0,
        y,
        width: 10.0,
        height,
    }
}

fn shift_at_scale(part: &KeptBox, to: BoxRect, scale: f32) -> Option<Shift> {
    let edges = |at| [at, at];
    Shift::of(
        0,
        (part, &Content::Element(super::tests::NODE)),
        from(to),
        (scale, edges),
    )
}

#[test]
fn a_half_pixel_edge_crossing_zero_at_1_5x_is_painted_again() {
    // Given: a 25 px box at y -26 (-39 device pixels), its bottom at
    // -1.5, which rounds to -2
    let part = kept(painting(tall(-26.0, 25.0)), tall(-26.0, 25.0));

    // When: it moves 28 px (42 device pixels) to y 2, bottom 40.5,
    // which rounds to 41, not -2 + 42
    let shift = shift_at_scale(&part, tall(2.0, 25.0), 1.5);

    // Then
    assert!(shift.is_none());
}

#[test]
fn a_half_pixel_edge_crossing_zero_at_1_25x_is_painted_again() {
    // Given: a 10 px box at y -12 (-15 device pixels), bottom -2.5
    let part = kept(painting(tall(-12.0, 10.0)), tall(-12.0, 10.0));

    // When: it moves 16 px (20 device pixels), bottom 17.5 -> 18, not
    // -3 + 20
    let shift = shift_at_scale(&part, tall(4.0, 10.0), 1.25);

    // Then
    assert!(shift.is_none());
}

#[test]
fn a_half_pixel_edge_staying_above_zero_at_1_5x_is_shifted() {
    // Given: bottom 40.5 -> 41, then 82.5 -> 83 = 41 + 42
    let part = kept(painting(tall(2.0, 25.0)), tall(2.0, 25.0));

    // When
    let shift = shift_at_scale(&part, tall(30.0, 25.0), 1.5);

    // Then
    assert!(shift.is_some());
}

#[test]
fn a_content_box_edge_crossing_zero_at_1_5x_is_painted_again() {
    // Given: a content box inset 1 px; its top at y -2 + 1 is -1.5
    // device pixels, which rounds to -2, while the border box's edges
    // are whole
    let part = kept(painting(tall(-2.0, 10.0)), tall(-2.0, 10.0));
    let inset = |at: BoxRect| {
        let content = BoxRect {
            y: at.y + 1.0,
            height: at.height - 2.0,
            ..at
        };
        [at, content]
    };
    let node = super::tests::NODE;

    // When: 28 px (42 device pixels) down, the top is 40.5, rounding
    // to 41, not -2 + 42
    let shift = Shift::of(
        0,
        (&part, &Content::Element(node)),
        from(tall(26.0, 10.0)),
        (1.5, inset),
    );

    // Then
    assert!(shift.is_none());
}

#[test]
fn a_part_with_a_range_track_is_painted_again() {
    // Given
    let mut painting = painting(tall(0.0, 10.0));
    painting.ranges.insert(
        NodeId(8),
        crate::range_paint::Track {
            x: 0.0,
            width: 10.0,
            thumb: 4.0,
        },
    );
    let part = kept(painting, tall(0.0, 10.0));

    // Then
    assert!(shift_of(&part, from(tall(24.0, 10.0))).is_none());
}

#[test]
fn a_part_with_a_text_fields_origin_is_painted_again() {
    // Given
    let mut painting = painting(tall(0.0, 10.0));
    painting.text_origins.insert(NodeId(8), [1.0, 1.0]);
    let part = kept(painting, tall(0.0, 10.0));

    // Then
    assert!(shift_of(&part, from(tall(24.0, 10.0))).is_none());
}
