//! GPU tier: the closed-form pipeline draws rects and ellipses with
//! coverage from the distance to the true outline in device pixels, the
//! ramp straddling it: 0.5 on the outline, half a pixel of ramp outside
//! as well as inside (Tom, 2026-11-13: as Chromium draws an edge).

#[path = "support/closed_forms.rs"]
mod support;

use support::{assert_at, circle, coverage, rect, scaled, white};

/// The edges of a rect from x = 10.75 to 20.25 and y = 1.5 to 7, in
/// device pixels, as each scale draws them.
fn assert_rect_edges_straddle(coverage: &[f32]) {
    assert_at(coverage, (9, 4), 0.0, "1.25 outside the left edge");
    assert_at(
        coverage,
        (10, 4),
        0.25,
        "0.25 outside: a fragment outside the outline",
    );
    assert_at(coverage, (11, 4), 1.0, "inside");
    assert_at(coverage, (20, 4), 0.25, "0.25 outside the right edge");
    assert_at(coverage, (15, 1), 0.5, "on the top edge");
    assert_at(coverage, (15, 0), 0.0, "above it");
}

#[test]
fn gpu_a_rect_at_1x_is_half_covered_on_its_outline_and_ramps_outside_it() {
    // When
    let rect = rect([10.75, 1.5], [9.5, 5.5], 0.0);
    let coverage = coverage(&[white(&rect, None, scaled(1.0))]);

    // Then
    assert_rect_edges_straddle(&coverage);
}

#[test]
fn gpu_a_rect_at_2x_ramps_over_one_device_pixel_not_one_unit() {
    // Given: the same rect in units half a device pixel each
    let rect = rect([5.375, 0.75], [4.75, 2.75], 0.0);
    let coverage = coverage(&[white(&rect, None, scaled(2.0))]);

    // Then
    assert_rect_edges_straddle(&coverage);
}

#[test]
fn gpu_a_circle_is_covered_by_its_distance_in_device_pixels() {
    // Given: a circle of radius 6.25 about (12, 12.5) at 1x, and the same
    // at 2x
    for (scale, centre, radius) in [(1.0, [12.0, 12.5], 6.25), (2.0, [6.0, 6.25], 3.125)] {
        let coverage = coverage(&[white(&circle(centre, radius), None, scaled(scale))]);

        // Then: on row 12 (centre y 12.5), column 17's centre is 0.75
        // inside, column 18's 0.25 outside, column 19's 1.25 outside
        let what = |column: u32| format!("{scale}x, column {column}");
        assert_at(&coverage, (17, 12), 1.0, &what(17));
        assert_at(&coverage, (18, 12), 0.25, &what(18));
        assert_at(&coverage, (19, 12), 0.0, &what(19));
        assert_at(&coverage, (12, 12), 1.0, &what(12));
    }
}

#[test]
fn gpu_a_later_shape_draws_over_an_earlier_one() {
    // Given: a rect, then a circle over its right half drawn black
    let under = white(&rect([4.0, 4.0], [16.0, 8.0], 2.0), None, scaled(1.0));
    let black = vapors_scene::Color::from_rgba8(0, 0, 0, 255);
    let over =
        vapors_renderer::ClosedFormShape::new(&circle([20.0, 8.0], 4.0), None, scaled(1.0), black)
            .expect("drawable")
            .expect("has area");

    // When
    let coverage = coverage(&[under, over]);

    // Then
    assert_at(&coverage, (8, 8), 1.0, "the rect");
    assert_at(&coverage, (19, 8), 0.0, "the circle over it");
}
