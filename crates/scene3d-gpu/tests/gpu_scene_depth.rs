//! The depth buffer: a nearer mesh hides a farther one whatever order
//! they are drawn in.

mod support;

use support::{Fixture, GREEN, RED, SIZE, at, centroid, render};
use vapors_scene3d::{Projection, Transform, Vec3};

#[test]
fn gpu_a_nearer_mesh_drawn_first_hides_a_farther_one_drawn_after_it() {
    // Given: a small green square at z = 1, added (and drawn) before a
    // larger red one behind it at z = 0
    let gpu = vapors_gpu_harness::gpu();
    let fixture = Fixture::new();
    fixture.camera(
        Projection::perspective(1.0, 0.1, 100.0).unwrap(),
        Vec3::new(0.0, 0.0, 5.0),
    );
    let root = fixture.root();
    let (small, large) = (fixture.square(0.3), fixture.square(0.8));
    fixture.mesh(
        root,
        &small,
        GREEN,
        Transform::from_translation(Vec3::Z).unwrap(),
    );
    fixture.mesh(root, &large, RED, Transform::IDENTITY);
    let mut renderer = fixture.renderer(&gpu);

    // When
    let pixels = render(&mut renderer, &gpu).expect("renders");

    // Then: green at the centre, red around it
    assert_eq!(at(&pixels, SIZE.width / 2, SIZE.height / 2), GREEN);
    let (_, red) = centroid(&pixels, RED);
    assert!(red > 0, "the farther square is drawn around the nearer one");
}

#[test]
fn gpu_the_backing_is_transparent_where_no_mesh_draws() {
    // Given
    let gpu = vapors_gpu_harness::gpu();
    let fixture = Fixture::new();
    fixture.camera(
        Projection::orthographic(6.0, 0.1, 100.0).unwrap(),
        Vec3::new(0.0, 0.0, 5.0),
    );
    let mut renderer = fixture.renderer(&gpu);

    // When
    let pixels = render(&mut renderer, &gpu).expect("renders");

    // Then
    assert_eq!(at(&pixels, 0, 0), [0, 0, 0, 0]);
}
