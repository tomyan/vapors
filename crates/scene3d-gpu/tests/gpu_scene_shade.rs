//! A mesh's colour times each vertex's shade (docs/plans/one-scene.md
//! D4, row R1): two faces of one mesh, shades 0.65 and 1.0, under one
//! colour, draw at 0.65 and 1.0 of it.

mod support;

use support::{Fixture, at, render};
use vapors_scene3d::{Colour, NodeKind, Projection, Transform, Vec3};

/// Two unit squares side by side in one geometry: the left at shade
/// 0.65, the right at 1.0.
fn two_faces(fixture: &Fixture) -> vapors_scene3d::GeometryHandle {
    let square = |x: f32| [(x, -0.5), (x + 1.0, -0.5), (x + 1.0, 0.5), (x, 0.5)];
    let positions = (square(-1.0).into_iter().chain(square(0.0)))
        .map(|(x, y)| Vec3::new(x, y, 0.0))
        .collect();
    let indices = vec![0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7];
    let shades = vec![0.65, 0.65, 0.65, 0.65, 1.0, 1.0, 1.0, 1.0];
    fixture
        .meshes
        .borrow_mut()
        .geometry(positions, indices, shades)
        .unwrap()
}

#[test]
fn gpu_two_faces_of_one_mesh_draw_their_shades_of_its_colour() {
    // Given: a red mesh, 20 pixels a unit
    let gpu = vapors_gpu_harness::gpu();
    let fixture = Fixture::new();
    let projection = Projection::orthographic(6.0, 0.1, 100.0).unwrap();
    fixture.camera(projection, Vec3::new(0.0, 0.0, 5.0));
    let colour = Colour::srgb(1.0, 0.0, 0.0).unwrap();
    let kind = NodeKind::Mesh {
        geometry: two_faces(&fixture),
        colour,
    };
    let root = fixture.root();
    fixture
        .scene
        .borrow_mut()
        .add(root, kind, Transform::IDENTITY)
        .unwrap();
    let mut renderer = fixture.renderer(&gpu);

    // When
    let pixels = render(&mut renderer, &gpu).expect("renders");

    // Then: the left face at 0.65 of 255, the right at 255
    assert_eq!(at(&pixels, 70, 60), [166, 0, 0, 255]);
    assert_eq!(at(&pixels, 90, 60), [255, 0, 0, 255]);
}
