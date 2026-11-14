//! GPU tier: where the mesh pipeline knowingly differs from the exact
//! answer, pinned so a change to either shows. Each is a known limit in
//! one-scene 1.2 that S3b measures against Chromium, retiring it with
//! its choice of method per kind (MSAA, or the exact corner method
//! written into its row) or recording it as accepted.

#[path = "support/closed_forms.rs"]
mod closed;
#[path = "support/meshes.rs"]
mod support;

use support::{assert_at, coverage, mesh, white};
use vapors_scene::Color;

#[test]
fn gpu_a_rotated_rects_corners_differ_from_the_closed_form_until_s3b() {
    // Given: a 12 x 8 rect rotated 30 degrees, as a mesh and as the
    // closed form
    let (sin, cos) = 30f32.to_radians().sin_cos();
    let transform = [cos, sin, -sin, cos, 16.0, 16.0];
    let meshed = coverage(&[white(mesh("M-6 -4h12v8h-12z", None), transform)]);
    let rect = closed::rect([-6.0, -4.0], [12.0, 8.0], 0.0);
    let exact = closed::coverage(&[closed::white(&rect, None, transform)]);

    // Then: two corners, where the triangle across the diagonal carries
    // only one of the corner's outline edges, are over-covered: inside
    // by the wedge limit (1 for about 0.82) and just outside (0.255 for
    // about 0.196); every other pixel matches within two 8-bit steps
    let corners = [(22, 15), (23, 15), (8, 16), (9, 16)];
    for (at, (m, c)) in (0u32..).zip(meshed.iter().zip(&exact)) {
        let pixel = (at % 32, at / 32);
        if !corners.contains(&pixel) {
            assert!(
                (m - c).abs() <= 2.0 / 255.0,
                "{pixel:?}: mesh {m}, closed form {c}"
            );
        }
    }
    assert_at(&meshed, (22, 15), 1.0, "inside a corner, the wedge limit");
    assert_at(&meshed, (9, 16), 1.0, "inside the other corner");
    assert_at(&meshed, (23, 15), 0.255, "just outside a corner");
    assert_at(&meshed, (8, 16), 0.255, "just outside the other");
}

#[test]
fn gpu_a_translucent_self_crossing_stroke_blends_its_overlap_twice_until_s3b() {
    // Given: two strokes crossing at the centre in one item, white at
    // half alpha over black
    let cross = mesh("M4 4L28 28M28 4L4 28", Some(4.0));
    let half = Color::from_rgba8(255, 255, 255, 128);

    // When
    let coverage = coverage(&[(cross, [1.0, 0.0, 0.0, 1.0, 0.0, 0.0], half)]);

    // Then: the overlap is blended twice (1 - (1 - a)^2), where SVG
    // paints the union once (a)
    let a = 128.0 / 255.0;
    assert_at(&coverage, (8, 8), a, "one stroke alone");
    assert_at(
        &coverage,
        (15, 15),
        1.0 - (1.0 - a) * (1.0 - a),
        "the crossing, twice",
    );
}
