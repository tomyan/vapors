//! GPU tier: where the mesh pipeline knowingly differs from the exact
//! answer, pinned so a change to either shows. Each is a divergence
//! recorded as accepted in one-scene 1.2.

#[path = "support/closed_forms.rs"]
mod closed;
#[path = "support/meshes.rs"]
mod support;

use support::{assert_at, coverage, mesh, white};
use vapors_scene::Color;

#[test]
fn gpu_a_rotated_rect_matches_the_closed_form_but_just_beyond_two_corners() {
    // Given: a 12 x 8 rect rotated 30 degrees, as a mesh and as the
    // closed form
    let (sin, cos) = 30f32.to_radians().sin_cos();
    let transform = [cos, sin, -sin, cos, 16.0, 16.0];
    let rect = closed::rect([-6.0, -4.0], [12.0, 8.0], 0.0);

    // When
    let meshed = coverage(&[white(mesh("M-6 -4h12v8h-12z", None), transform)]);
    let exact = closed::coverage(&[closed::white(&rect, None, transform)]);

    // Then: the pixel just beyond each of two corners takes the farther
    // of the corner's two lines (0.255), where the closed form takes the
    // distance to the corner (0.196); every other pixel, the corners'
    // insides included, within two 8-bit steps
    let beyond = [(23, 15), (8, 16)];
    for (at, (m, c)) in (0u32..).zip(meshed.iter().zip(&exact)) {
        let pixel = (at % 32, at / 32);
        if !beyond.contains(&pixel) {
            assert!(
                (m - c).abs() <= 2.0 / 255.0,
                "{pixel:?}: mesh {m}, closed form {c}"
            );
        }
    }
    assert_at(&meshed, (23, 15), 0.255, "just beyond a corner");
    assert_at(&meshed, (8, 16), 0.255, "just beyond the other");
}

#[test]
fn gpu_a_translucent_self_crossing_stroke_blends_its_overlap_twice() {
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
