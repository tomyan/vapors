//! GPU tier: a mesh under a group with a negative scale (determinant -1,
//! as the racer's world group in one-scene 2.2) draws. Its triangles'
//! winding turns over on screen, so a renderer that adds back-face
//! culling or lit normals must flip front-face and the normal matrix for
//! such a node (flat-scene.md, F8 and F9); this test fails if one culls
//! without doing so.

use super::support::materials::{RED, looking_down};
use super::support::{SIZE, at, render};
use vapors_scene3d::{Quat, Transform, Vec3};

#[test]
fn gpu_a_mesh_under_a_mirroring_group_draws() {
    // Given: a square inside a group mirroring X
    let gpu = vapors_gpu_harness::gpu();
    let fixture = looking_down();
    let mirror = Transform::new(Vec3::ZERO, Quat::IDENTITY, Vec3::new(-1.0, 1.0, 1.0)).unwrap();
    let group = fixture.group(mirror);
    let square = fixture.square(1.0);
    fixture.mesh(group, &square, RED, Transform::IDENTITY);
    let mut renderer = fixture.renderer(&gpu);

    // When
    let pixels = render(&mut renderer, &gpu).expect("renders");

    // Then
    assert_eq!(at(&pixels, SIZE.width / 2, SIZE.height / 2), RED);
}
