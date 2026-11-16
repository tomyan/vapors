//! A mesh whose corners lie behind the eye keeps an exact ramp along the
//! edges that run from there into view (decal D2b): a corner at or
//! behind the eye has no wedge on the screen, and the corners in front
//! carry its edges' lines, each stretched past any fragment where its far
//! end is behind the eye.

#[path = "support/meshes.rs"]
mod page;
#[path = "support/projective.rs"]
mod scene;
#[path = "support/projective_meshes.rs"]
mod scene_meshes;

use glam::{Mat4, Vec3};
use page::{SIDE, mesh, white};
use scene::{on_screen, pixel_centre};
use scene_meshes::mesh_coverage_3d;

/// The signed distance from `p` to the line through `a` and `b`,
/// positive on the side `inside` lies.
fn to_line(p: [f32; 2], [a, b]: [[f32; 2]; 2], inside: [f32; 2]) -> f32 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let across = |q: [f32; 2]| ((q[0] - a[0]) * dy - (q[1] - a[1]) * dx) / dx.hypot(dy);
    across(p) * across(inside).signum()
}

#[test]
fn gpu_a_strip_from_behind_the_eye_ramps_by_its_distance_along_its_sides() {
    // Given a strip 0.6 wide on the ground under an eye 1 unit up, running
    // from 5 units behind the eye to 50 ahead: its near corners behind
    let lens = vapors_scene3d::Projection::perspective(45f32.to_radians(), 0.1, 100.0)
        .and_then(|lens| lens.matrix(1.0))
        .expect("a valid lens");
    let world = Mat4::from_translation(Vec3::new(0.0, -1.0, 0.0))
        * Mat4::from_rotation_x(-90f32.to_radians());
    let strip = mesh("M-0.3 -5H0.3V50H-0.3Z", None);

    // When
    let drawn = mesh_coverage_3d(
        SIDE,
        &[white(strip, [1.0, 0.0, 0.0, 1.0, 0.0, 0.0])],
        (lens, world),
    );

    // Then each side, as two points in front of the eye project it, gives
    // every pixel beside it its ramp, in the rows below the far end
    let screen = |q: [f32; 2]| on_screen(SIDE, (lens, world), q);
    let sides = [
        [screen([-0.3, 1.0]), screen([-0.3, 40.0])],
        [screen([0.3, 1.0]), screen([0.3, 40.0])],
    ];
    let middle = screen([0.0, 2.0]);
    let mut judged = 0;
    for (at, found) in drawn.iter().enumerate() {
        let p = pixel_centre(at, SIDE);
        let inside = sides
            .iter()
            .map(|side| to_line(p, *side, middle))
            .fold(f32::INFINITY, f32::min);
        if p[1] < 20.0 || inside.abs() > 0.5 {
            continue;
        }
        judged += 1;
        let expected = (0.5 + inside).clamp(0.0, 1.0);
        assert!(
            (found - expected).abs() <= 0.01,
            "pixel {p:?} covered {found}; 0.5 plus its distance inside is {expected}"
        );
    }
    assert!(
        judged > 10,
        "{judged} pixels beside the strip's sides judged"
    );
}
