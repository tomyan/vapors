//! A node's uniform follows its world matrix whoever runs
//! `Scene::update`: the renderer compares each node's world version
//! with the one it last wrote, not the nodes its own update returned.

mod support;

use support::{Fixture, RED, centroid, render};
use vapors_scene3d::{Projection, Transform, Vec3};

#[test]
fn gpu_a_node_another_caller_updated_draws_at_its_new_place() {
    // Given: a square drawn left of centre, then moved right, and the
    // scene updated by someone other than the renderer
    let gpu = vapors_gpu_harness::gpu();
    let fixture = Fixture::new();
    let projection = Projection::orthographic(6.0, 0.1, 100.0).unwrap();
    fixture.camera(projection, Vec3::new(0.0, 0.0, 5.0));
    let square = fixture.square(0.5);
    let left = Transform::from_translation(Vec3::new(-2.0, 0.0, 0.0)).unwrap();
    let node = fixture.mesh(fixture.root(), &square, RED, left);
    let mut renderer = fixture.renderer(&gpu);
    render(&mut renderer, &gpu).expect("renders");
    let right = Transform::from_translation(Vec3::new(2.0, 0.0, 0.0)).unwrap();
    let mut scene = fixture.scene.borrow_mut();
    scene.set_transform(node, right).unwrap();
    scene.update();
    drop(scene);

    // When
    let pixels = render(&mut renderer, &gpu).expect("renders");

    // Then: 20 pixels a unit, so x = 2 is at pixel 80 + 40
    let ([x, _], count) = centroid(&pixels, RED);
    assert!(count > 20, "the square covers {count} pixels");
    assert!((x - 120.0).abs() <= 1.0, "the square is centred at x {x}");
}
