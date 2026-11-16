//! Closed forms on a plane at 60 degrees ramp by their distance on the
//! screen (#422's review, H1, for D1): within half a device pixel of the
//! outline, each pixel's coverage is 0.5 minus its distance to the
//! projected outline, which `support/screen_distance.rs` takes with no
//! Jacobian. This pins the fragment stage's map from device pixels to
//! shape units, which a box-filter oracle's allowance hides.

#[path = "support/closed_forms.rs"]
mod page;
#[path = "support/projective.rs"]
mod scene;
#[path = "support/screen_distance.rs"]
mod screen;

use glam::{Mat4, Vec3};
use page::{SIDE, circle, rect, scaled, white};
use scene::{coverage_3d, rounded_rect_distance};
use screen::{Outline, circle_outline, ramp_by_distance, rounded_rect_outline};

/// How far a band pixel of the circle may be from 0.5 minus its screen
/// distance: the ramp takes the distance in shape units over the map at
/// the fragment, exact to first order across the half-pixel band;
/// measured on 2026-11-16 at most 0.0223 with the analytic map (0.039
/// with this adapter's fine derivatives), and the readback's 8 bits.
const CIRCLE_DISTANCE: f32 = 0.025;

/// The same for the stroked frame, whose inner corners' radius (1 shape
/// unit, 0.65 device pixel) bends within the band, so the distance's
/// first-order step parts from the outline more: measured 0.046 with the
/// analytic map (0.041 with derivatives, so this case does not tell the
/// two apart; the circle does), and 8 bits.
const FRAME_DISTANCE: f32 = 0.05;

fn turned_plane() -> (Mat4, Mat4) {
    let lens = vapors_scene3d::Projection::perspective(45f32.to_radians(), 0.1, 100.0)
        .and_then(|lens| lens.matrix(1.0))
        .expect("a valid lens");
    let world = Mat4::from_translation(Vec3::new(0.0, 0.0, -3.0))
        * Mat4::from_rotation_x(60f32.to_radians());
    (lens, world)
}

/// A case: its name, the shape, its outlines, whether a shape point is
/// in it, and its bound.
type Case<'a> = (
    &'static str,
    vapors_renderer::ClosedFormShape,
    Vec<Outline>,
    &'a dyn Fn([f32; 2]) -> bool,
    f32,
);

#[test]
fn gpu_closed_forms_at_60_degrees_ramp_by_their_distance_on_the_screen() {
    let small = scaled(0.05);
    let in_circle = |p: [f32; 2]| (p[0] + 6.0).hypot(p[1] - 4.0) <= 12.0;
    let on_frame =
        |p: [f32; 2]| rounded_rect_distance(p, [4.0, -14.0], [14.0, 10.0], 3.0).abs() <= 2.0;
    let frame_edges = vec![
        rounded_rect_outline([4.0, -14.0], [14.0, 10.0], 3.0, 2.0),
        rounded_rect_outline([4.0, -14.0], [14.0, 10.0], 3.0, -2.0),
    ];
    let cases: [Case<'_>; 2] = [
        (
            "the circle",
            white(&circle([-6.0, 4.0], 12.0), None, small),
            vec![circle_outline([-6.0, 4.0], 12.0)],
            &in_circle,
            CIRCLE_DISTANCE,
        ),
        (
            "the stroked frame",
            white(&rect([4.0, -14.0], [14.0, 10.0], 3.0), Some(4.0), small),
            frame_edges,
            &on_frame,
            FRAME_DISTANCE,
        ),
    ];
    for (what, shape, outlines, inside, bound) in cases {
        // When
        let drawn = coverage_3d(SIDE, &[shape], turned_plane().0, turned_plane().1);
        let expected = ramp_by_distance(SIDE, (&outlines, inside), small, turned_plane());

        // Then
        let judged: Vec<(f32, f32)> = drawn
            .iter()
            .zip(&expected)
            .filter_map(|(d, e)| e.map(|e| (*d, e)))
            .collect();
        let worst = judged
            .iter()
            .map(|(d, e)| (d - e).abs())
            .fold(0.0, f32::max);
        assert!(
            judged.len() > 20,
            "{what}: {} band pixels judged",
            judged.len()
        );
        assert!(
            worst <= bound,
            "{what}: a band pixel is {worst} from 0.5 minus its screen distance"
        );
    }
}
