//! The scene's closed-form vertex stage pads each shape's quad by about
//! one device pixel as the projection draws it (docs/plans/decal.md
//! section 2, D1 review M1): every pixel centre within half a device
//! pixel outside the shape gets a fragment, for the outer half of its
//! ramp, and none past one and a half does, under a steep perspective
//! where the projection's own term in the Jacobian is what decides.

#[path = "support/footprint.rs"]
mod footprint;
#[path = "support/closed_forms.rs"]
mod page;
#[path = "support/projective.rs"]
mod scene;

use footprint::{footprint, outside_quad};
use glam::{Mat4, Vec3};
use page::{SIDE, rect, white};
use scene::{on_screen, pixel_centre};

fn camera(degrees: f32, distance: f32) -> (Mat4, Mat4) {
    let lens = vapors_scene3d::Projection::perspective(45f32.to_radians(), 0.1, 100.0)
        .and_then(|lens| lens.matrix(1.0))
        .expect("a valid lens");
    let world = Mat4::from_translation(Vec3::new(0.0, 0.0, -distance))
        * Mat4::from_rotation_x(degrees.to_radians());
    (lens, world)
}

#[test]
fn gpu_a_steep_plane_pads_each_quad_between_half_and_one_and_a_half_device_pixels() {
    // Measured on 2026-11-16, D1 at 395083aa with this review (32 px target): the nearest pixel centre left
    // without a fragment was 0.76 to 1.02 px outside and the farthest
    // with one 0.38 to 1.07 px, over these planes. Without the
    // projection's term in the Jacobian (`- clip.xy * m[k].w`) the
    // 85 degree planes leave one at 0.23 px and draw one at 2.27 px.
    for (degrees, distance) in [(60.0, 3.0), (80.0, 2.0), (85.0, 2.0), (85.0, 1.5)] {
        // Given a rect across the plane, its far half near the horizon
        let (origin, size) = ([-1.2, -1.6], [2.4, 2.4]);
        let shape = white(
            &rect(origin, size, 0.0),
            None,
            [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
        );
        let view = camera(degrees, distance);

        // When
        let drawn = footprint(SIDE, &[shape], view);

        // Then: a fragment for every centre within the ramp's outer half,
        // and none past the 1 px margin and half a pixel of rasterising
        let [x0, y0] = origin;
        let [x1, y1] = [x0 + size[0], y0 + size[1]];
        let corners = [[x0, y0], [x1, y0], [x1, y1], [x0, y1]].map(|q| on_screen(SIDE, view, q));
        for (at, value) in drawn.iter().enumerate() {
            let centre = pixel_centre(at, SIDE);
            let out = outside_quad(corners, centre);
            let what = format!("{degrees} degrees: pixel {centre:?}, {out} px outside");
            assert!(out > 0.5 || *value > 0.5, "{what}, has no fragment");
            assert!(out <= 1.5 || *value < 0.5, "{what}, has a fragment");
        }
    }
}
