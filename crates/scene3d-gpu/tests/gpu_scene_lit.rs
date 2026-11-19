//! A lit box against the CPU's formula (docs/plans/lighting.md, LT3a):
//! ambient and Lambert in linear light, encoded back to sRGB.

mod support;

use support::lit::{box_mesh, encoded, shining, sun};
use support::{Fixture, render};
use vapors_scene3d::{Ambient, Colour, Projection, Quat, Shading, Transform, Vec3};

const ALBEDO: [u8; 4] = [204, 128, 77, 255];

/// The 8-bit colour of a face with `normal` lit by white from `to_light`
/// and `ambient`: the formula `lighting.wgsl` is held to.
fn expected(normal: Vec3, to_light: Vec3, ambient: f32) -> [f32; 3] {
    let lambert = normal.dot(to_light).max(0.0);
    encoded(ALBEDO, ambient + lambert, 0.0)
}

/// A box turned to show three faces: its transform and their normals.
fn shown_faces() -> (Transform, Vec<Vec3>) {
    let turn = Quat::from_rotation_x(0.6) * Quat::from_rotation_y(0.5);
    let at = Transform::new(Vec3::ZERO, turn, Vec3::splat(2.0)).unwrap();
    let axes = [Vec3::X, Vec3::Y, Vec3::Z, -Vec3::X, -Vec3::Y, -Vec3::Z];
    let shown = axes.map(|axis| turn * axis).into_iter();
    (at, shown.filter(|normal| normal.z > 0.05).collect())
}

/// How many pixels are within 1/255 per channel of `want`.
fn matching(pixels: &[u8], want: [f32; 3]) -> usize {
    let near = |p: &&[u8; 4]| (0..3).all(|i| (f32::from(p[i]) - want[i]).abs() <= 1.0);
    pixels.as_chunks::<4>().0.iter().filter(near).count()
}

#[test]
fn gpu_a_box_lit_by_one_directional_light_shows_each_face_as_the_cpu_formula() {
    // Given: a box, white ambient 0.1, and a light from the upper right
    let gpu = vapors_gpu_harness::gpu();
    let fixture = Fixture::new();
    let projection = Projection::orthographic(6.0, 0.1, 100.0).unwrap();
    fixture.camera(projection, Vec3::new(0.0, 0.0, 5.0));
    let (at, faces) = shown_faces();
    box_mesh(&fixture, ALBEDO, at, Shading::Lit);
    let to_light = Vec3::new(0.5, 0.8, 0.6).normalize();
    sun(&fixture, shining(-to_light));
    let white = Colour::srgb(1.0, 1.0, 1.0).unwrap();
    let ambient = Ambient::new(white, 0.1).unwrap();
    fixture.scene.borrow_mut().set_ambient(ambient);
    let mut renderer = fixture.renderer(&gpu);

    // When
    let pixels = render(&mut renderer, &gpu).expect("renders");

    // Then: each shown face is a patch of the formula's colour
    assert_eq!(faces.len(), 3);
    let wants: Vec<[f32; 3]> = (faces.iter())
        .map(|&normal| expected(normal, to_light, 0.1))
        .collect();
    for (normal, want) in faces.iter().zip(&wants) {
        let count = matching(&pixels, *want);
        assert!(count > 300, "face {normal}: {count} pixels of {want:?}");
    }
}
