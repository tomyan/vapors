//! GPU tier: every mesh pixel more than half a device pixel inside its
//! outline is fully covered, and none more than half a pixel outside is
//! covered at all (#371 review, M2 and H1): a notched rect, as lyon
//! triangulates it and as the review did, whose corner lines reach past
//! the ends of their edges into the fill; touche's icons; strokes with
//! an acute join, a join just under the miter limit and one over it;
//! and a mirrored polygon.

#[path = "support/mesh_oracle.rs"]
mod oracle;
#[path = "support/meshes.rs"]
mod support;

use support::{assert_at, assert_ramp_within_half_a_pixel, coverage, mesh, white};
use vapors_shapes::{MeshTriangle, mesh_triangles};

fn check(triangles: &[MeshTriangle], transform: [f32; 6], what: &str) {
    let coverage = coverage(&[white(triangles.to_vec(), transform)]);
    assert_ramp_within_half_a_pixel(&coverage, triangles, transform, what);
}

/// The ramp's band against the CPU drawing, for each subpath of `data`
/// drawn alone: where two subpaths overlap, the pipeline blends one ramp
/// over the other, which a union's area differs from by up to a quarter
/// (S3b's known overlap limit), so overlaps are left to `check`'s rule.
fn check_band(data: &str, stroke: Option<f32>, transform: [f32; 6], what: &str) {
    assert!(
        !data.contains('m'),
        "{what}: split at absolute moves only; a relative move needs its start"
    );
    for start in data.match_indices('M').map(|(at, _)| at) {
        let end = data[start + 1..]
            .find('M')
            .map_or(data.len(), |at| start + 1 + at);
        let subpath = mesh(&data[start..end], stroke);
        let coverage = coverage(&[white(subpath.clone(), transform)]);
        oracle::assert_band_matches_cpu(&coverage, &subpath, transform, what);
    }
}

/// The notched rect `M0 0H1V1H2V0H3V3H2.1H0Z` at 8x, its notch's right
/// side at x = 20.25, so pixel centres fall a quarter pixel from it.
const NOTCHED: [f32; 6] = [8.0, 0.0, 0.0, 8.0, 4.25, 2.25];

#[test]
fn gpu_a_notched_rect_as_the_review_triangulated_it_is_full_inside() {
    // Given: vertices 0 to 8 round the notched rect, and the review's
    // fan, in which triangles at (2, 0) reach up past the notch's top
    let vertices = [
        [0.0, 0.0],
        [1.0, 0.0],
        [1.0, 1.0],
        [2.0, 1.0],
        [2.0, 0.0],
        [3.0, 0.0],
        [3.0, 3.0],
        [2.1, 3.0],
        [0.0, 3.0],
    ];
    let indices = [
        4, 5, 6, 4, 6, 7, 4, 7, 3, 3, 7, 8, 3, 8, 2, 2, 8, 0, 2, 0, 1,
    ];
    let triangles = mesh_triangles(&vertices, &indices).expect("the review's mesh builds");

    // When, Then
    check(&triangles, NOTCHED, "the review's notched rect");
    let coverage = coverage(&[white(triangles.clone(), NOTCHED)]);
    oracle::assert_band_matches_cpu(&coverage, &triangles, NOTCHED, "the review's notched rect");
}

#[test]
fn gpu_a_notched_rect_as_lyon_triangulates_it_is_full_inside() {
    check(
        &mesh("M0 0H1V1H2V0H3V3H2.1H0Z", None),
        NOTCHED,
        "lyon's notched rect",
    );
    check_band(
        "M0 0H1V1H2V0H3V3H2.1H0Z",
        None,
        NOTCHED,
        "lyon's notched rect",
    );
}

#[test]
fn gpu_touches_icons_at_2x_ramp_within_half_a_pixel() {
    let icons = [
        "M1.5 4.5v8h13v-7h-6l-1.5-1.5h-5.5z",
        "M5 3H3v11h10V3h-2M5.5 2h5v2.5h-5z",
        "M8 10V2M5 5l3-3 3 3M3 9v5h10V9",
        "M5 3L2 6l3 3M2 6h7a4 4 0 0 1 0 8H6",
        "M11 3l3 3-3 3M14 6H7a4 4 0 0 0 0 8h3",
        "M2 4h12M2 8h12M2 12h12M5 2v4M11 6v4M7 10v4",
        "M3 10l5-5 5 5",
        "M2 6v4M5 4v8M8 2v12M11 4v8M14 6v4",
        "M2 6h3l4-3v10l-4-3H2zM11 6l3 4M14 6l-3 4",
        "M2 8h11M9 4l4 4-4 4",
    ];
    for data in icons {
        check(&mesh(data, Some(1.5)), [2.0, 0.0, 0.0, 2.0, 0.0, 0.0], data);
        check_band(data, Some(1.5), [2.0, 0.0, 0.0, 2.0, 0.0, 0.0], data);
    }
}

#[test]
fn gpu_the_open_icons_overlapping_inner_join_ramps_within_half_a_pixel() {
    // Given: touche's Open icon, where lyon's stroker leaves triangles
    // overlapping round the inner join at (2.25, 4.75); the walk calls
    // it reflex, the outline's turn convex, and the relaxed guard in
    // crates/shapes/src/mesh_wedge.rs keeps reflex (one-scene 1.2)
    let open = mesh("M1.5 4.5v8h13v-7h-6l-1.5-1.5h-5.5z", Some(1.5));

    // When, Then
    check(&open, [2.0, 0.0, 0.0, 2.0, 0.0, 0.0], "Open");
}

#[test]
fn gpu_strokes_joins_ramp_within_half_a_pixel() {
    let joins = [
        ("M4 28L16 4L28 28", "an acute join, mitered"),
        ("M9.8 28L16 4L22.2 28", "a join just under the miter limit"),
        ("M12 28L16 4L20 28", "a join over the miter limit, bevelled"),
    ];
    for (data, what) in joins {
        check(&mesh(data, Some(3.0)), [1.0, 0.0, 0.0, 1.0, 0.0, 0.0], what);
        check_band(data, Some(3.0), [1.0, 0.0, 0.0, 1.0, 0.0, 0.0], what);
    }
}

#[test]
fn gpu_a_mirrored_polygon_ramps_within_half_a_pixel() {
    // Given: scale(-1, 1), whose determinant is negative, moved down a
    // quarter pixel so the top edge lies at y = 4.25
    let triangles = mesh("M4 4L28 4L16 14L28 28L4 28z", None);
    let mirrored = [-1.0, 0.0, 0.0, 1.0, 32.0, 0.25];

    // When
    let coverage = coverage(&[white(triangles.clone(), mirrored)]);

    // Then: the ramp holds, and the top edge straddles its line: the
    // pixel centre 0.25 inside it is covered 0.75, which a corner moved
    // in rather than out would leave uncovered
    assert_ramp_within_half_a_pixel(&coverage, &triangles, mirrored, "mirrored polygon");
    oracle::assert_band_matches_cpu(&coverage, &triangles, mirrored, "mirrored polygon");
    assert_at(
        &coverage,
        (16, 4),
        0.75,
        "0.25 inside the mirrored top edge",
    );
}

#[test]
fn gpu_the_outer_ramp_beside_a_near_straight_corner_is_not_cut() {
    // Given touche's Redo icon at 1.55: its top edge runs straight into
    // the arc at (9.05, 5.25), a corner turning 8 degrees, and from x =
    // 8.5 to there the pixels just outside the edge read 0 where the CPU
    // drawing gives a third (found by decal D2's 8x comparison)
    let redo = "M5 3L2 6l3 3M2 6h7a4 4 0 0 1 0 8H6";

    // When, Then
    for transform in [
        [1.55, 0.0, 0.0, 1.55, 3.3, 2.5],
        [1.55, 0.0, 0.0, 0.83, 3.3, 6.3],
    ] {
        check_band(redo, Some(1.5), transform, "Redo at 1.55");
    }
}
