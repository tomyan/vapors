//! GPU tier: no mesh pixel is covered more than half a device pixel
//! outside its true outline (#360 review, H1): at the corners of a
//! convex and a concave polygon, round a circle flattened under
//! scale(10, 1), and round a skewed rect, where triangles moved out by
//! the extrusion carry only some of a corner's outline edges.

#[path = "support/meshes.rs"]
mod support;

use support::{assert_nothing_past_half_a_pixel, coverage, mesh, scaled, white};

fn check(data: &str, transform: [f32; 6], what: &str) {
    let triangles = mesh(data, None);
    let coverage = coverage(&[white(triangles.clone(), transform)]);
    assert_nothing_past_half_a_pixel(&coverage, &triangles, transform, what);
}

#[test]
fn gpu_a_convex_polygons_corners_cover_nothing_past_half_a_pixel() {
    check("M3 5L25 3L28 20L9 27z", scaled(1.0), "convex polygon");
}

#[test]
fn gpu_a_concave_polygons_corners_cover_nothing_past_half_a_pixel() {
    check(
        "M4 4L28 4L16 14L28 28L4 28z",
        scaled(1.0),
        "concave polygon",
    );
}

#[test]
fn gpu_a_circle_flattened_under_scale_10_1_covers_nothing_past_half_a_pixel() {
    check(
        "M0.4 1.6a1.2 1.2 0 1 0 2.4 0a1.2 1.2 0 1 0-2.4 0z",
        [10.0, 0.0, 0.0, 1.0, 0.0, 14.0],
        "flattened circle",
    );
}

#[test]
fn gpu_a_skewed_rect_covers_nothing_past_half_a_pixel() {
    check(
        "M2 2h10v8h-10z",
        [1.0, 0.0, 1.0, 1.0, 4.0, 4.0],
        "skewed rect",
    );
}

#[test]
fn gpu_a_wide_rects_corners_cover_nothing_past_half_a_pixel() {
    // The review's case: a unit square under matrix(10, 0, 0, 1, 8, 7.8)
    check("M0 0h1v1h-1z", [10.0, 0.0, 0.0, 1.0, 8.0, 7.8], "wide rect");
}

#[test]
fn gpu_a_skewed_meshs_slanted_edge_ramps_its_full_half_pixel_outside() {
    // Given: matrix(1, 0, 0.5, 1, 0, 0), so the left edge x = 4 runs
    // along x' = 4 + 0.5 y', and its device normal is not its own
    let skew = [1.0, 0.0, 0.5, 1.0, 0.0, 0.0];
    let coverage = coverage(&[white(mesh("M4.25 2.25h6v6h-6z", None), skew)]);

    // Then: (5.5, 3.5), near the top corner, is 0.5 / sqrt 1.25 = 0.447
    // px outside the slanted edge, near the end of the ramp; the corner's
    // move must reach it across the edge's device normal, not its own
    let out = 0.5 / 1.25f32.sqrt();
    support::assert_at(
        &coverage,
        (5, 3),
        0.5 - out,
        "0.447 outside the slanted edge",
    );
    support::assert_at(&coverage, (9, 2), 0.75, "0.25 inside the top edge");
}
