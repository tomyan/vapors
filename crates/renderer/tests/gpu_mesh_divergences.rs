//! GPU tier: where the mesh pipeline knowingly differs from SVG, pinned
//! so a change shows: the divergence recorded as accepted in one-scene
//! 1.2.

#[path = "support/meshes.rs"]
mod support;

use support::{assert_at, coverage, mesh};
use vapors_scene::Color;

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
