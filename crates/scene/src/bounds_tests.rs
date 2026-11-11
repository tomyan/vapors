use super::*;
use crate::{Color, Quad};

#[test]
fn a_shadow_is_drawn_three_standard_deviations_past_its_shape() {
    // Given: a CSS blur of 4, so a standard deviation of 2
    let rect = Rect::new(10.0, 10.0, 20.0, 20.0);
    let shadow = Shadow {
        rect,
        radii: [0.0; 4],
        blur: 4.0,
        color: Color::default(),
        caster: rect,
        caster_radii: [0.0; 4],
    };

    // When
    let drawn = shadow.drawn();

    // Then
    assert_eq!(drawn, Rect::new(4.0, 4.0, 26.0, 26.0));
}

#[test]
fn an_items_bounds_are_its_rect_with_a_pixel_of_fringe() {
    // Given
    let quad = Item::Quad(Quad::fill(
        Rect::new(10.0, 10.0, 20.0, 20.0),
        Color::default(),
    ));

    // When
    let bounds = quad.bounds();

    // Then
    assert_eq!(bounds, Rect::new(9.0, 9.0, 21.0, 21.0));
}

#[test]
fn rectangles_that_meet_at_an_edge_are_disjoint() {
    // Given
    let left = Rect::new(0.0, 0.0, 10.0, 10.0);
    let right = Rect::new(10.0, 0.0, 20.0, 10.0);

    // Then
    assert!(left.is_disjoint(&right));
    assert!(!left.is_disjoint(&right.grown(0.5)));
}

#[test]
fn a_nan_edge_makes_a_rectangle_neither_empty_nor_disjoint_on_its_side() {
    // Given: a rectangle left of the other's NaN edge
    let unplaced = Rect::new(f32::NAN, 0.0, 10.0, 10.0);
    let far = Rect::new(-600.0, 0.0, -500.0, 10.0);

    // Then
    assert!(!unplaced.is_empty());
    assert!(!unplaced.is_disjoint(&far));
}
