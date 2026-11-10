//! What a render issues through `SurfaceFrame::work`: geometry the
//! first time a piece is drawn, a mesh node's uniform only when its
//! world transform changed, the camera's only when it changed, and one
//! draw per mesh node.

mod support;

use support::{Fixture, GREEN, RED, render_counted};
use vapors_app::GpuWork;
use vapors_gpu_harness::Gpu;
use vapors_scene3d::{NodeId, Projection, Transform, Vec3};
use vapors_scene3d_gpu::SceneRenderer;

/// A mesh node's uniform: its world matrix and colour, 20 floats.
const MESH_BYTES: u64 = 80;
/// The camera's view-projection, 16 floats.
const CAMERA_BYTES: u64 = 64;
/// The test square: 4 positions of 3 floats and 6 `u32` indices.
const SQUARE_BYTES: u64 = 4 * 12 + 6 * 4;

/// Two squares sharing one piece of geometry, a camera, and a renderer
/// that has drawn them once; the first square's node, and that frame's work.
fn drawn_once(gpu: &Gpu) -> (Fixture, SceneRenderer, NodeId, GpuWork) {
    let fixture = Fixture::new();
    let projection = Projection::orthographic(6.0, 0.1, 100.0).unwrap();
    fixture.camera(projection, Vec3::new(0.0, 0.0, 5.0));
    let square = fixture.square(0.5);
    let left = Transform::from_translation(Vec3::new(-2.0, 0.0, 0.0)).unwrap();
    let node = fixture.mesh(fixture.root(), square, RED, left);
    fixture.mesh(fixture.root(), square, GREEN, Transform::IDENTITY);
    let mut renderer = fixture.renderer(gpu);
    let (_, first) = render_counted(&mut renderer, gpu).expect("renders");
    (fixture, renderer, node, first)
}

#[test]
fn gpu_the_first_frame_uploads_shared_geometry_once_and_writes_every_uniform() {
    // Given
    let gpu = vapors_gpu_harness::gpu();

    // When
    let (_, _, _, first) = drawn_once(&gpu);

    // Then
    let bytes = SQUARE_BYTES + 2 * MESH_BYTES + CAMERA_BYTES;
    assert_eq!(first.buffer_bytes, bytes);
    assert_eq!(first.texture_bytes, 0);
    assert_eq!(first.draw_calls, 2);
}

#[test]
fn gpu_a_frame_with_nothing_changed_writes_nothing_and_draws_each_mesh() {
    // Given
    let gpu = vapors_gpu_harness::gpu();
    let (_, mut renderer, _, _) = drawn_once(&gpu);

    // When
    let (_, idle) = render_counted(&mut renderer, &gpu).expect("renders");

    // Then
    assert_eq!(idle.upload_bytes(), 0);
    assert_eq!(idle.draw_calls, 2);
}

#[test]
fn gpu_moving_one_node_writes_its_uniform_alone() {
    // Given
    let gpu = vapors_gpu_harness::gpu();
    let (fixture, mut renderer, node, _) = drawn_once(&gpu);
    let right = Transform::from_translation(Vec3::new(2.0, 0.0, 0.0)).unwrap();
    fixture
        .scene
        .borrow_mut()
        .set_transform(node, right)
        .unwrap();

    // When
    let (_, moved) = render_counted(&mut renderer, &gpu).expect("renders");

    // Then
    assert_eq!(moved.buffer_bytes, MESH_BYTES);
    assert_eq!(moved.texture_bytes, 0);
    assert_eq!(moved.draw_calls, 2);
}

#[test]
fn gpu_moving_the_camera_writes_its_uniform_alone() {
    // Given
    let gpu = vapors_gpu_harness::gpu();
    let (fixture, mut renderer, _, _) = drawn_once(&gpu);
    let camera = fixture.scene.borrow().active_camera().unwrap();
    let eye = Transform::from_translation(Vec3::new(1.0, 0.0, 5.0)).unwrap();
    fixture
        .scene
        .borrow_mut()
        .set_transform(camera, eye)
        .unwrap();

    // When
    let (_, moved) = render_counted(&mut renderer, &gpu).expect("renders");

    // Then
    assert_eq!(moved.buffer_bytes, CAMERA_BYTES);
}

#[test]
fn gpu_a_mesh_added_later_uploads_its_geometry_and_writes_its_uniform_alone() {
    // Given
    let gpu = vapors_gpu_harness::gpu();
    let (fixture, mut renderer, _, _) = drawn_once(&gpu);
    let bigger = fixture.square(1.0);
    let up = Transform::from_translation(Vec3::new(0.0, 2.0, 0.0)).unwrap();
    fixture.mesh(fixture.root(), bigger, RED, up);

    // When
    let (_, added) = render_counted(&mut renderer, &gpu).expect("renders");

    // Then
    assert_eq!(added.buffer_bytes, SQUARE_BYTES + MESH_BYTES);
    assert_eq!(added.draw_calls, 3);
}
