//! Geometry with normals draws unlit exactly as without (docs/plans/
//! lighting.md, LT1): the unlit pipeline ignores the normals buffer.

mod support;

use support::{Fixture, RED, render};
use vapors_scene3d::{Projection, Quat, Transform, Vec3, flat_shaded};

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

/// The box turned to show three faces, drawn from geometry with or
/// without normals.
fn drawn(with_normals: bool) -> Vec<u8> {
    let gpu = vapors_gpu_harness::gpu();
    let fixture = Fixture::new();
    let projection = Projection::orthographic(6.0, 0.1, 100.0).unwrap();
    fixture.camera(projection, Vec3::new(0.0, 0.0, 5.0));
    let (positions, indices) = box_triangles();
    let flat = flat_shaded(&positions, &indices, &[1.0; 8]).unwrap();
    let mut meshes = fixture.meshes.borrow_mut();
    let geometry = if with_normals {
        meshes.lit_geometry(flat.positions, flat.indices, flat.shades, flat.normals)
    } else {
        meshes.geometry(flat.positions, flat.indices, flat.shades)
    }
    .unwrap();
    drop(meshes);
    let turn = Quat::from_rotation_x(0.6) * Quat::from_rotation_y(0.5);
    let at = Transform::new(Vec3::ZERO, turn, Vec3::splat(2.0)).unwrap();
    fixture.mesh(fixture.root(), &geometry, RED, at);
    let mut renderer = fixture.renderer(&gpu);
    render(&mut renderer, &gpu).expect("renders")
}

#[test]
fn gpu_a_geometry_with_normals_draws_unlit_the_same_pixels() {
    let plain = drawn(false);
    let with = drawn(true);
    let covered = plain
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|p| **p == RED)
        .count();
    assert!(covered > 500, "the box covers {covered} pixels");
    assert!(
        plain == with,
        "pixels differ when the geometry carries normals"
    );
}
