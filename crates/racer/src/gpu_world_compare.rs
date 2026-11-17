//! What `gpu_world_tests.rs` compares: the Track's flat pixels against
//! the scene's, and where `Projection::project` puts the car's roof.

use vapors_app::SurfaceSize;
use vapors_scene3d::{Mat4, Vec3, project};

use crate::gpu_world_rig::{Rig, size_f32};
use crate::model::Race;
use crate::world::{car_transform, racers_world};

/// How far the scene's pixel may be from the Track's flat one, a
/// channel: the ground's noise hashes `floor(q)`, and the two pipelines'
/// `q` differ in the last bits, so a pixel at a noise cell's edge can
/// take the next cell's value. The noise spans 0.018 (5 levels); road
/// pixels 2 to 3 apart were measured on 2026-11-17 when the zoom moved
/// from the camera's scale to the lens.
const APART: u8 = 6;

/// How many of the Track's flat pixels there are (a pixel and its eight
/// neighbours within 2 a channel, so no edge), and how many of them the
/// scene's differ from by more than `APART` a channel.
pub fn agreement(size: SurfaceSize, track: &[u8], scene: &[u8]) -> (usize, usize) {
    let (width, height) = (size.width as usize, size.height as usize);
    let at = |pixels: &[u8], x: usize, y: usize| -> [u8; 4] {
        let start = (y * width + x) * 4;
        pixels[start..start + 4].try_into().expect("four bytes")
    };
    let within = |most: u8| {
        move |a: [u8; 4], b: [u8; 4]| a.iter().zip(b).all(|(a, b)| a.abs_diff(b) <= most)
    };
    let (flat_around, close) = (within(2), within(APART));
    let (mut flat, mut apart) = (0, 0);
    for y in 1..height - 1 {
        for x in 1..width - 1 {
            let centre = at(track, x, y);
            let around = (0..9).map(|k| at(track, x + k % 3 - 1, y + k / 3 - 1));
            if around.into_iter().all(|other| flat_around(centre, other)) {
                flat += 1;
                apart += usize::from(!close(centre, at(scene, x, y)));
            }
        }
    }
    (flat, apart)
}

/// The pixel `project` puts the car roof's centre on, in the rig's scene.
pub fn roof_pixel(rig: &Rig, race: &Race) -> (u32, u32) {
    let local = Vec3::new(-0.5, 0.0, 15.0);
    let world: Mat4 = racers_world().matrix() * car_transform(race).unwrap().matrix();
    let (width, height) = size_f32(rig.size);
    let matrix = rig.scene.borrow().view_projection(width / height);
    let ndc = project(matrix.expect("a camera"), world.transform_point3(local));
    let ndc = ndc.expect("orthographic: every point projects");
    let pixel = |unit: f32, side: f32| -> u32 {
        let at = (unit * side).floor();
        assert!(
            (0.0..side).contains(&at),
            "the roof is in view: {at} of {side}"
        );
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a whole number in 0..side, checked"
        )]
        let at = at as u32;
        at
    };
    (
        pixel(ndc.x.midpoint(1.0), width),
        pixel((1.0 - ndc.y) * 0.5, height),
    )
}
