//! A mesh item: its extrusion is half a pixel along the direction its
//! transform shrinks most; a transform with no area draws nothing; one
//! not finite is refused.

use vapors_scene::Color;

use super::MeshShape;

fn extrusion(transform: [f32; 6]) -> f32 {
    let shape = MeshShape::new(transform, Color::from_rgba8(1, 2, 3, 255))
        .expect("drawable")
        .expect("has area");
    shape.instance()[6]
}

#[test]
fn the_extrusion_is_half_a_pixel_in_the_direction_the_transform_shrinks_most() {
    // When, Then: half a pixel is 0.5 units at 1x, 0.25 at 2x, and 0.25
    // along x under a scale of (2, 3); a rotation keeps lengths
    assert!((extrusion([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]) - 0.5).abs() < 1e-6);
    assert!((extrusion([2.0, 0.0, 0.0, 2.0, 5.0, 6.0]) - 0.25).abs() < 1e-6);
    assert!((extrusion([2.0, 0.0, 0.0, 3.0, 0.0, 0.0]) - 0.25).abs() < 1e-6);
    let (sin, cos) = 30f32.to_radians().sin_cos();
    assert!((extrusion([cos, sin, -sin, cos, 0.0, 0.0]) - 0.5).abs() < 1e-5);
}

#[test]
fn a_skew_extrudes_by_its_smaller_singular_value() {
    // Given: matrix(1, 0, 1, 1): singular values (sqrt 5 +- 1) / 2
    let smaller = (5f32.sqrt() - 1.0) / 2.0;

    // When, Then
    assert!((extrusion([1.0, 0.0, 1.0, 1.0, 0.0, 0.0]) - 0.5 / smaller).abs() < 1e-5);
}

#[test]
fn a_transform_with_no_area_draws_nothing_and_one_not_finite_is_refused() {
    // When, Then
    let red = Color::from_rgba8(255, 0, 0, 255);
    assert_eq!(MeshShape::new([0.0; 6], red), Ok(None));
    let refused =
        MeshShape::new([f32::INFINITY, 0.0, 0.0, 1.0, 0.0, 0.0], red).expect_err("refused");
    assert!(
        refused
            .0
            .starts_with("transform [inf, 0.0, 0.0, 1.0, 0.0, 0.0] is not finite"),
        "{refused}"
    );
}
