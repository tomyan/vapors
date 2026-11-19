//! A lit box and its lights, for the lit material's tests.

use super::{Fixture, SIZE, at, to_pixel};
use vapors_scene3d::{
    Colour, Light, NodeId, NodeKind, Quat, Shading, Transform, Vec3, flat_shaded, project,
};

/// A unit box's twelve triangles, counter-clockwise from outside.
fn box_triangles() -> (Vec<Vec3>, Vec<u32>) {
    let corner = |i: u8| {
        let bit = |b: u8| f32::from((i >> b) & 1) - 0.5;
        Vec3::new(bit(0), bit(1), bit(2))
    };
    let indices = [
        0, 2, 1, 1, 2, 3, 4, 5, 6, 5, 7, 6, 0, 1, 4, 1, 5, 4, 2, 6, 3, 3, 6, 7, 0, 4, 2, 2, 4, 6,
        1, 3, 5, 3, 7, 5,
    ];
    ((0..8).map(corner).collect(), indices.to_vec())
}

/// A flat-shaded box with normals under the root at `at`: its node.
pub fn box_mesh(fixture: &Fixture, rgba: [u8; 4], at: Transform, shading: Shading) -> NodeId {
    let (positions, indices) = box_triangles();
    let flat = flat_shaded(&positions, &indices, &[1.0; 8]).unwrap();
    let geometry = fixture
        .meshes
        .borrow_mut()
        .lit_geometry(flat.positions, flat.indices, flat.shades, flat.normals)
        .unwrap();
    let node = fixture.mesh(fixture.root(), &geometry, rgba, at);
    fixture
        .scene
        .borrow_mut()
        .set_shading(node, shading)
        .unwrap();
    node
}

/// A white directional light of intensity 1 under the root at `at`.
pub fn sun(fixture: &Fixture, at: Transform) -> NodeId {
    let colour = Colour::srgb(1.0, 1.0, 1.0).unwrap();
    let light = Light::Directional {
        colour,
        intensity: 1.0,
    };
    let root = fixture.root();
    let mut scene = fixture.scene.borrow_mut();
    scene.add(root, NodeKind::Light(light), at).unwrap()
}

/// The turn a node needs to shine along `direction`.
pub fn shining(direction: Vec3) -> Transform {
    let turn = Quat::from_rotation_arc(-Vec3::Z, direction.normalize());
    Transform::new(Vec3::ZERO, turn, Vec3::ONE).unwrap()
}

fn encode(c: f32) -> f32 {
    if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

/// The 8-bit colour `lighting.wgsl` is held to for `albedo` (sRGB):
/// its linear colour times `scale` (ambient plus diffuse, the lights'
/// colour being white) plus a white `spec`, encoded back to sRGB.
pub fn encoded(albedo: [u8; 4], scale: f32, spec: f32) -> [f32; 3] {
    let [r, g, b, _] = albedo.map(|c| f32::from(c) / 255.0);
    let linear = Colour::srgb(r, g, b).unwrap().linear();
    linear.map(|a| (255.0 * encode(a * scale + spec)).min(255.0))
}

/// The rendered colour at the pixel `world` projects to.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // inside the backing
pub fn colour_at(pixels: &[u8], fixture: &Fixture, world: Vec3) -> [u8; 4] {
    #[allow(clippy::cast_precision_loss)]
    let aspect = SIZE.width as f32 / SIZE.height as f32;
    let matrix = fixture.scene.borrow().view_projection(aspect).unwrap();
    let [x, y] = to_pixel(project(matrix, world).expect("in front of the eye"));
    at(pixels, x as u32, y as u32)
}

/// Whether `got` is within 1/255 per channel of `want`.
pub fn near(got: [u8; 4], want: [f32; 3]) -> bool {
    (0..3).all(|i| (f32::from(got[i]) - want[i]).abs() <= 1.0)
}
