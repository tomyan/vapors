//! Frames the scene renderer refuses, naming the fix.

mod support;

use support::{Fixture, RED, render};
use vapors_scene3d::{Colour, Meshes, NodeKind, Projection, Transform, Vec3};

#[test]
fn gpu_a_mesh_naming_geometry_from_another_meshes_is_refused_naming_the_node() {
    // Given
    let gpu = vapors_gpu_harness::gpu();
    let fixture = Fixture::new();
    fixture.camera(
        Projection::orthographic(6.0, 0.1, 100.0).unwrap(),
        Vec3::new(0.0, 0.0, 5.0),
    );
    let foreign = Meshes::new()
        .geometry(
            vec![Vec3::ZERO, Vec3::X, Vec3::Y],
            vec![0, 1, 2],
            vec![1.0; 3],
        )
        .unwrap();
    let root = fixture.root();
    let colour = Colour::srgb(1.0, 0.0, 0.0).unwrap();
    let kind = NodeKind::Mesh {
        geometry: foreign.clone(),
        colour,
    };
    let node = fixture
        .scene
        .borrow_mut()
        .add(root, kind, Transform::IDENTITY)
        .unwrap();
    let mut renderer = fixture.renderer(&gpu);

    // When
    let refusal = render(&mut renderer, &gpu)
        .expect_err("refused")
        .to_string();

    // Then
    assert!(
        refusal.contains(&format!("mesh {node} names {foreign}")),
        "{refusal}"
    );
    assert!(refusal.contains("SceneRenderer::new"), "{refusal}");
}

#[test]
fn gpu_a_scene_with_no_active_camera_is_refused_naming_the_fix() {
    // Given
    let gpu = vapors_gpu_harness::gpu();
    let fixture = Fixture::new();
    let square = fixture.square(0.5);
    fixture.mesh(fixture.root(), &square, RED, Transform::IDENTITY);
    let mut renderer = fixture.renderer(&gpu);

    // When
    let refusal = render(&mut renderer, &gpu)
        .expect_err("refused")
        .to_string();

    // Then
    assert!(refusal.contains("Scene::set_camera"), "{refusal}");
}
