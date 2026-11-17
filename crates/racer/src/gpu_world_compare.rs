//! What `gpu_world_tests.rs` compares: the scene's pixels against the
//! golden pictures' flat ones, and where `Projection::project` puts the
//! car's roof.

use vapors_app::SurfaceSize;
use vapors_scene3d::{Mat4, Vec3, project};

use crate::gpu_world_rig::{Rig, size_f32};
use crate::model::Race;
use crate::world::{car_transform, racers_world};

/// How the scene's pixels compare with the reference's flat ones (a
/// pixel and its eight neighbours within 2 a channel, so no edge).
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Agreement {
    /// The reference's flat pixels.
    pub flat: usize,
    /// Those the scene's differs from by 3 to 6 at most a channel: the
    /// ground's noise, which hashes `floor(q)`, takes the next cell's
    /// value at a cell's edge when the two pipelines' `q` differ in the
    /// last bits, or across GPUs (the noise spans 5 levels).
    pub noisy: usize,
    /// Those the scene's differs from by more than 6 a channel.
    pub apart: usize,
}

/// The scene's pixels against the reference's flat ones.
pub fn agreement(size: SurfaceSize, reference: &[u8], scene: &[u8]) -> Agreement {
    let (width, height) = (size.width as usize, size.height as usize);
    let at = |pixels: &[u8], x: usize, y: usize| -> [u8; 4] {
        let start = (y * width + x) * 4;
        pixels[start..start + 4].try_into().expect("four bytes")
    };
    let most = |a: [u8; 4], b: [u8; 4]| {
        a.iter()
            .zip(b)
            .map(|(a, b)| a.abs_diff(b))
            .max()
            .unwrap_or(0)
    };
    let mut found = Agreement::default();
    for y in 1..height - 1 {
        for x in 1..width - 1 {
            let centre = at(reference, x, y);
            let mut around = (0..9).map(|k| at(reference, x + k % 3 - 1, y + k / 3 - 1));
            if around.all(|other| most(centre, other) <= 2) {
                found.flat += 1;
                match most(centre, at(scene, x, y)) {
                    0..=2 => {}
                    3..=6 => found.noisy += 1,
                    _ => found.apart += 1,
                }
            }
        }
    }
    found
}

/// The pixel `project` puts the car roof's centre on, in the rig's scene.
pub fn roof_pixel(rig: &Rig, race: &Race) -> (u32, u32) {
    let local = Vec3::new(-0.5, 0.0, 15.0);
    let world: Mat4 = racers_world().matrix() * car_transform(race).unwrap().matrix();
    let (width, height) = size_f32(rig.size);
    let matrix = rig.world.scene().borrow().view_projection(width / height);
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
