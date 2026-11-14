//! GPU tier: coverage under transforms whose columns differ (a
//! non-uniform scale, a skew), a rotation, and strokes scaled with the
//! shape: the distance in device pixels across each edge, and a quad
//! grown far enough past each (#345 review, M1 and M2).

#[path = "support/closed_forms.rs"]
mod support;

use support::{assert_at, coverage, rect, scaled, white};

#[test]
fn gpu_a_rect_scaled_three_times_wider_ramps_one_pixel_across_each_edge() {
    // Given: x from 11.25 to 17.25, y from 2.1 to 8.1 in device pixels
    let wide = rect([3.75, 2.1], [2.0, 6.0], 0.0);
    let coverage = coverage(&[white(&wide, None, [3.0, 0.0, 0.0, 1.0, 0.0, 0.0])]);

    // Then
    assert_at(&coverage, (10, 5), 0.0, "0.75 outside the left edge");
    assert_at(&coverage, (11, 5), 0.75, "0.25 inside the left edge");
    assert_at(&coverage, (17, 5), 0.25, "0.25 outside the right edge");
    assert_at(&coverage, (14, 2), 0.9, "0.4 inside the top edge");
    // The quad must reach 0.4 px below the bottom edge for this one.
    assert_at(&coverage, (14, 8), 0.1, "0.4 outside the bottom edge");
}

#[test]
fn gpu_a_skewed_rects_slanted_edge_ramps_across_its_perpendicular() {
    // Given: matrix(1, 0, 0.5, 1, 0, 0), so the left edge x = 4 runs
    // along x' = 4 + 0.5 y'
    let skewed = rect([4.0, 2.25], [6.0, 6.0], 0.0);
    let coverage = coverage(&[white(&skewed, None, [1.0, 0.0, 0.5, 1.0, 0.0, 0.0])]);

    // Then: (6.5, 5.5) is 0.25 left of the edge along x, 0.25 / sqrt
    // 1.25 = 0.224 across it
    assert_at(
        &coverage,
        (6, 5),
        0.5 - 0.25 / 1.25f32.sqrt(),
        "0.224 outside the slanted edge",
    );
    assert_at(&coverage, (7, 5), 1.0, "0.67 inside it");
    assert_at(&coverage, (9, 2), 0.75, "0.25 inside the top edge");
}

#[test]
fn gpu_a_rotated_rect_covers_its_area() {
    // Given: a 12 x 8 rect rotated 30 degrees about the target's centre;
    // a ramp inside the outline only would lose half a pixel along all
    // 40 pixels of it, 20 of 96
    let (sin, cos) = 30f32.to_radians().sin_cos();
    let turned = rect([-6.0, -4.0], [12.0, 8.0], 0.0);
    let coverage = coverage(&[white(&turned, None, [cos, sin, -sin, cos, 16.0, 16.0])]);

    // Then
    let area: f32 = coverage.iter().sum();
    assert!((area - 96.0).abs() < 1.0, "covered {area} of 96");
    assert_at(&coverage, (16, 16), 1.0, "the centre");
}

#[test]
fn gpu_a_stroke_covers_half_its_width_either_side_of_the_outline_at_any_scale() {
    // Given: a rect from x = 8 stroked 2 wide in device pixels, at 1x and
    // as half-pixel units at 2x
    let one = white(&rect([8.0, 8.0], [16.0, 16.0], 0.0), Some(2.0), scaled(1.0));
    let two = white(&rect([4.0, 4.0], [8.0, 8.0], 0.0), Some(1.0), scaled(2.0));
    for (scale, shape) in [(1, one), (2, two)] {
        let coverage = coverage(&[shape]);

        // Then
        let what = |part: &str| format!("{scale}x, {part}");
        assert_at(&coverage, (6, 16), 0.0, &what("left of the stroke"));
        assert_at(&coverage, (7, 16), 1.0, &what("the stroke's outer half"));
        assert_at(&coverage, (8, 16), 1.0, &what("the stroke's inner half"));
        assert_at(&coverage, (9, 16), 0.0, &what("inside the stroke"));
    }
}

#[test]
fn gpu_a_strokes_fractional_edges_straddle_too() {
    // Given: a 2.5 wide stroke on x = 8, from 6.75 to 9.25
    let stroked = white(&rect([8.0, 8.0], [16.0, 16.0], 0.0), Some(2.5), scaled(1.0));

    // When
    let coverage = coverage(&[stroked]);

    // Then
    assert_at(&coverage, (6, 16), 0.25, "0.25 outside the outer edge");
    assert_at(&coverage, (9, 16), 0.25, "0.25 inside the inner edge");
}
