//! A box lit by a point light, against the CPU's formula (docs/plans/
//! lighting.md, `LT3b`): per-pixel direction and the smooth falloff to
//! zero at `range`, in linear light.

mod support;

use support::lit::{box_mesh, colour_at, encoded, near};
use support::{Fixture, render};
use vapors_scene3d::{
    Ambient, Colour, Light, NodeKind, Projection, Quat, Shading, Transform, Vec3,
};

const ALBEDO: [u8; 4] = [204, 128, 77, 255];
const AMBIENT: f32 = 0.1;
const LAMP: Vec3 = Vec3::new(0.5, 0.3, 3.0);
const RANGE: f32 = 2.5;

/// The formula for the front face (normal +Z, at z = 1) at `(x, y, 1)`.
fn expected(x: f32, y: f32) -> [f32; 3] {
    let offset = LAMP - Vec3::new(x, y, 1.0);
    let inside = (1.0 - offset.length_squared() / (RANGE * RANGE)).clamp(0.0, 1.0);
    let lambert = offset.normalize().z.max(0.0);
    encoded(ALBEDO, AMBIENT + inside * inside * lambert, 0.0)
}

#[test]
fn gpu_a_box_lit_by_a_point_light_shows_the_cpu_formula_at_each_probe() {
    // Given: a box, white ambient 0.1 and a lamp 2 units off its front face
    let gpu = vapors_gpu_harness::gpu();
    let fixture = Fixture::new();
    let projection = Projection::orthographic(6.0, 0.1, 100.0).unwrap();
    fixture.camera(projection, Vec3::new(0.0, 0.0, 5.0));
    let scale = Transform::new(Vec3::ZERO, Quat::IDENTITY, Vec3::splat(2.0)).unwrap();
    box_mesh(&fixture, ALBEDO, scale, Shading::Lit);
    let white = Colour::srgb(1.0, 1.0, 1.0).unwrap();
    let lamp = Light::Point {
        colour: white,
        intensity: 1.0,
        range: RANGE,
    };
    {
        let mut scene = fixture.scene.borrow_mut();
        let root = scene.root();
        let at = Transform::from_translation(LAMP).unwrap();
        scene.add(root, NodeKind::Light(lamp), at).unwrap();
        scene.set_ambient(Ambient::new(white, AMBIENT).unwrap());
    }
    let mut renderer = fixture.renderer(&gpu);

    // When
    let pixels = render(&mut renderer, &gpu).expect("renders");

    // Then: under the lamp, off to each side, and past its range
    for (x, y) in [
        (0.5, 0.3),
        (-0.8, -0.6),
        (0.8, 0.7),
        (0.0, 0.0),
        (-0.9, -0.9),
    ] {
        let got = colour_at(&pixels, &fixture, Vec3::new(x, y, 1.0));
        let want = expected(x, y);
        assert!(near(got, want), "at ({x}, {y}): {got:?}, wanted {want:?}");
    }
}
