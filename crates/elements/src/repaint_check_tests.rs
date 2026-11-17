//! The outline ring's reaches from its border box (`repaint_check.rs`).

use vapors_scene::Rect;

use super::ring_grown_evenly;

/// A 90 x 30 border box at the origin.
fn border_box() -> Rect {
    Rect::new(0.0, 0.0, 90.0, 30.0)
}

#[test]
fn a_ring_grown_by_the_same_reach_on_every_side_is_even() {
    assert!(ring_grown_evenly(
        Rect::new(-4.5, -4.5, 94.5, 34.5),
        border_box()
    ));
}

#[test]
fn a_ring_grown_more_on_one_side_is_not_even() {
    assert!(!ring_grown_evenly(
        Rect::new(-4.5, -4.5, 95.0, 34.5),
        border_box()
    ));
}

#[test]
fn a_nan_reach_from_a_finite_box_is_not_even() {
    assert!(!ring_grown_evenly(
        Rect::new(f32::NAN, -4.5, 94.5, 34.5),
        border_box()
    ));
}

#[test]
fn a_side_whose_box_edge_is_infinite_is_not_asked_for_a_reach() {
    // Given: the box's right edge infinite (issue #449), so the ring's is
    let wide = Rect::new(0.0, 0.0, f32::INFINITY, 30.0);

    // Then: the other three reaches are compared
    assert!(ring_grown_evenly(
        Rect::new(-4.5, -4.5, f32::INFINITY, 34.5),
        wide
    ));
}

#[test]
fn a_lone_reach_left_to_compare_must_still_be_finite() {
    // Given: every box edge infinite but the left, whose reach is NaN
    let wide = Rect::new(0.0, f32::NEG_INFINITY, f32::INFINITY, f32::INFINITY);
    let ring = Rect::new(f32::NAN, f32::NEG_INFINITY, f32::INFINITY, f32::INFINITY);

    // Then
    assert!(!ring_grown_evenly(ring, wide));
}

#[test]
fn an_infinite_box_edge_needs_the_same_infinity_on_the_ring() {
    // Given: the box's right edge infinite, the ring's finite
    let wide = Rect::new(0.0, 0.0, f32::INFINITY, 30.0);

    // Then
    assert!(!ring_grown_evenly(Rect::new(-4.5, -4.5, 94.5, 34.5), wide));
}

#[test]
fn a_finite_box_edge_with_an_infinite_ring_edge_is_not_even() {
    assert!(!ring_grown_evenly(
        Rect::new(-4.5, -4.5, f32::INFINITY, 34.5),
        border_box()
    ));
}

#[test]
fn a_nan_in_a_later_reach_is_not_even() {
    assert!(!ring_grown_evenly(
        Rect::new(-4.5, -4.5, 94.5, f32::NAN),
        border_box()
    ));
}

#[test]
fn an_infinite_left_edge_compares_the_other_three_reaches() {
    // Given: the left edge infinite, so the first reach compared is the
    // top's
    let wide = Rect::new(f32::NEG_INFINITY, 0.0, 90.0, 30.0);

    // Then
    assert!(ring_grown_evenly(
        Rect::new(f32::NEG_INFINITY, -4.5, 94.5, 34.5),
        wide
    ));
    assert!(!ring_grown_evenly(
        Rect::new(f32::NEG_INFINITY, -4.5, 95.0, 34.5),
        wide
    ));
}

#[test]
fn a_box_infinite_on_every_side_is_even_only_with_the_same_infinities() {
    let everywhere = Rect::new(
        f32::NEG_INFINITY,
        f32::NEG_INFINITY,
        f32::INFINITY,
        f32::INFINITY,
    );
    assert!(ring_grown_evenly(everywhere, everywhere));
    let flipped = Rect::new(
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::INFINITY,
        f32::INFINITY,
    );
    assert!(!ring_grown_evenly(flipped, everywhere));
}
