//! The lights uniform: written when a light moves or ambient changes,
//! and not otherwise; a scene without lights never writes it.

mod support;

use support::{Fixture, RED, render_counted};
use vapors_app::GpuWork;
use vapors_gpu_harness::Gpu;
use vapors_scene3d::{Ambient, Colour, Light, NodeId, NodeKind, Projection, Transform, Vec3};
use vapors_scene3d_gpu::SceneRenderer;

/// Ambient, the count, and eight lights of 48 bytes.
const LIGHTS_BYTES: u64 = 32 + 8 * 48;

/// A square, a camera and one point light; rendered once. The light's
/// node, and the first frame's work.
fn lit_once(gpu: &Gpu) -> (Fixture, SceneRenderer, NodeId, GpuWork) {
    let fixture = Fixture::new();
    fixture.camera(
        Projection::orthographic(6.0, 0.1, 100.0).unwrap(),
        Vec3::new(0.0, 0.0, 5.0),
    );
    let square = fixture.square(0.5);
    fixture.mesh(fixture.root(), &square, RED, Transform::IDENTITY);
    let lamp = NodeKind::Light(Light::Point {
        colour: Colour::srgb(1.0, 1.0, 1.0).unwrap(),
        intensity: 1.0,
        range: 4.0,
    });
    let root = fixture.root();
    let node = fixture
        .scene
        .borrow_mut()
        .add(root, lamp, Transform::IDENTITY)
        .unwrap();
    let mut renderer = fixture.renderer(gpu);
    let (_, first) = render_counted(&mut renderer, gpu).expect("renders");
    (fixture, renderer, node, first)
}

#[test]
fn gpu_the_first_frame_writes_the_lights_uniform_once_and_an_idle_frame_nothing() {
    // Given / When
    let gpu = vapors_gpu_harness::gpu();
    let (_, mut renderer, _, first) = lit_once(&gpu);
    let (_, idle) = render_counted(&mut renderer, &gpu).expect("renders");

    // Then: the square (4 vertices of 16 bytes, 6 indices), its slot (80)
    // and the camera (80), and the uniform
    assert_eq!(first.buffer_bytes, 88 + 80 + 80 + LIGHTS_BYTES);
    assert_eq!(idle.buffer_bytes, 0);
    assert_eq!(idle.draw_calls, 1);
}

#[test]
fn gpu_moving_a_light_writes_the_uniform_once_and_no_node_slots() {
    // Given
    let gpu = vapors_gpu_harness::gpu();
    let (fixture, mut renderer, lamp, _) = lit_once(&gpu);
    let to = Transform::from_translation(Vec3::new(1.0, 1.0, 1.0)).unwrap();

    // When
    fixture.scene.borrow_mut().set_transform(lamp, to).unwrap();
    let (_, moved) = render_counted(&mut renderer, &gpu).expect("renders");
    let (_, after) = render_counted(&mut renderer, &gpu).expect("renders");

    // Then
    assert_eq!(moved.buffer_bytes, LIGHTS_BYTES);
    assert_eq!(after.buffer_bytes, 0);
}

#[test]
fn gpu_changing_ambient_writes_the_uniform_once() {
    // Given
    let gpu = vapors_gpu_harness::gpu();
    let (fixture, mut renderer, _, _) = lit_once(&gpu);
    let grey = Colour::srgb(0.5, 0.5, 0.5).unwrap();

    // When
    let ambient = Ambient::new(grey, 0.2).unwrap();
    fixture.scene.borrow_mut().set_ambient(ambient);
    let (_, changed) = render_counted(&mut renderer, &gpu).expect("renders");
    let (_, same) = render_counted(&mut renderer, &gpu).expect("renders");

    // Then
    assert_eq!(changed.buffer_bytes, LIGHTS_BYTES);
    assert_eq!(same.buffer_bytes, 0);
}

#[test]
fn gpu_a_scene_without_lights_never_writes_the_uniform() {
    // Given: lit_once's scene less its light, as gpu_scene_counts has it
    let gpu = vapors_gpu_harness::gpu();
    let fixture = Fixture::new();
    fixture.camera(
        Projection::orthographic(6.0, 0.1, 100.0).unwrap(),
        Vec3::new(0.0, 0.0, 5.0),
    );
    let square = fixture.square(0.5);
    let mesh = fixture.mesh(fixture.root(), &square, RED, Transform::IDENTITY);
    let mut renderer = fixture.renderer(&gpu);

    // When
    let (_, first) = render_counted(&mut renderer, &gpu).expect("renders");
    let to = Transform::from_translation(Vec3::X).unwrap();
    fixture.scene.borrow_mut().set_transform(mesh, to).unwrap();
    let (_, moved) = render_counted(&mut renderer, &gpu).expect("renders");

    // Then: the square, its slot and the camera; then the slot alone
    assert_eq!(first.buffer_bytes, 88 + 80 + 80);
    assert_eq!(moved.buffer_bytes, 80);
}
