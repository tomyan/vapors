//! The racer's ground as an application material (one-scene D5 (b),
//! R2): `ground.wgsl`'s colours, drawn on a plane mesh by the scene
//! renderer, as the Track renderer draws them under its own projection
//! until R3a2 moves the racer to the scene.

use vapors_scene3d::Material;

use crate::model::{RADIUS, ROAD, STRAIGHT};

/// `ground.wgsl`, the track's constants put before it.
pub(crate) fn ground_wgsl() -> String {
    let constants = format!(
        "const TRACK_STRAIGHT: f32 = {STRAIGHT:?};\nconst TRACK_RADIUS: f32 = {RADIUS:?};\nconst TRACK_ROAD: f32 = {ROAD:?};\n"
    );
    [constants.as_str(), include_str!("ground.wgsl")].concat()
}

/// The ground, for a mesh in the scene's coordinates: the racer's world
/// under a root group that mirrors Y and turns -90 degrees about X
/// (one-scene 2.2), so a ground point's simulation X, Y is the scene's
/// X, Z. Edges ramp over 0.8 of the larger screen-axis footprint of a
/// device pixel (`fwidth`), which under the Track's straight-down view
/// is the Track's 0.8 of a pixel, and the target darkens toward its
/// corners by the Track's vignette.
#[must_use]
pub fn ground_material() -> Material {
    Material {
        label: "racer ground",
        wgsl: [ground_wgsl().as_str(), MATERIAL].concat(),
    }
}

const MATERIAL: &str = "
fn material(at: MaterialInput) -> vec4<f32> {
    let q = at.world.xz;
    let footprint = fwidth(q);
    let pixel = 0.8 * max(footprint.x, footprint.y);
    var c = ground(q, pixel);
    let uv = at.screen / at.size;
    c *= 1. - 0.13 * dot(uv - 0.5, uv - 0.5);
    return vec4<f32>(c, 1.);
}
";

#[cfg(test)]
mod tests {
    #[test]
    fn the_ground_reads_only_the_material_contract() {
        let held = vapors_scene3d::Meshes::new().material(super::ground_material());
        assert!(held.is_ok(), "{held:?}");
    }
}
