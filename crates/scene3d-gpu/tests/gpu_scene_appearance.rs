//! A mesh's appearance (`Scene::set_appearance`, docs/plans/css-for-3d.md
//! C3): another colour writes its uniform alone; hidden, it is not drawn
//! and nothing is written; shown again, it is drawn with nothing written.

mod support;

use support::{Fixture, GREEN, RED, centroid, render_counted};
use vapors_gpu_harness::Gpu;
use vapors_scene3d::{Colour, NodeId, Projection, Transform, Vec3};
use vapors_scene3d_gpu::SceneRenderer;

/// A mesh node's uniform: its world matrix and colour, 20 floats.
const MESH_BYTES: u64 = 80;
const BLUE: [u8; 4] = [0, 0, 255, 255];

/// A red square left of centre and a green one at the centre, drawn
/// once; the red square's node.
fn drawn_once(gpu: &Gpu) -> (Fixture, SceneRenderer, NodeId) {
    let fixture = Fixture::new();
    let projection = Projection::orthographic(6.0, 0.1, 100.0).unwrap();
    fixture.camera(projection, Vec3::new(0.0, 0.0, 5.0));
    let square = fixture.square(0.5);
    let left = Transform::from_translation(Vec3::new(-2.0, 0.0, 0.0)).unwrap();
    let node = fixture.mesh(fixture.root(), &square, RED, left);
    fixture.mesh(fixture.root(), &square, GREEN, Transform::IDENTITY);
    let mut renderer = fixture.renderer(gpu);
    render_counted(&mut renderer, gpu).expect("renders");
    (fixture, renderer, node)
}

fn set(fixture: &Fixture, node: NodeId, colour: Colour, visible: bool) {
    let mut scene = fixture.scene.borrow_mut();
    scene.set_appearance(node, colour, visible).unwrap();
}

#[test]
fn gpu_a_new_colour_writes_one_uniform_and_recolours_that_mesh() {
    // Given
    let gpu = vapors_gpu_harness::gpu();
    let (fixture, mut renderer, node) = drawn_once(&gpu);

    // When
    set(&fixture, node, Colour::srgb(0.0, 0.0, 1.0).unwrap(), true);
    let (pixels, work) = render_counted(&mut renderer, &gpu).expect("renders");

    // Then
    assert_eq!((work.buffer_bytes, work.draw_calls), (MESH_BYTES, 2));
    assert!(centroid(&pixels, BLUE).1 > 20 && centroid(&pixels, RED).1 == 0);
}

#[test]
fn gpu_a_hidden_mesh_is_not_drawn_and_nothing_is_written() {
    // Given
    let gpu = vapors_gpu_harness::gpu();
    let (fixture, mut renderer, node) = drawn_once(&gpu);
    let red = Colour::srgb(1.0, 0.0, 0.0).unwrap();

    // When
    set(&fixture, node, red, false);
    let (pixels, work) = render_counted(&mut renderer, &gpu).expect("renders");

    // Then
    assert_eq!((work.buffer_bytes, work.draw_calls), (0, 1));
    assert_eq!(centroid(&pixels, RED).1, 0);
    assert!(centroid(&pixels, GREEN).1 > 20);
}

#[test]
fn gpu_a_mesh_moved_while_hidden_and_shown_again_is_drawn_where_it_is() {
    // Given: hidden, moved right, then shown
    let gpu = vapors_gpu_harness::gpu();
    let (fixture, mut renderer, node) = drawn_once(&gpu);
    let red = Colour::srgb(1.0, 0.0, 0.0).unwrap();
    set(&fixture, node, red, false);
    render_counted(&mut renderer, &gpu).expect("renders");
    let right = Transform::from_translation(Vec3::new(2.0, 0.0, 0.0)).unwrap();
    fixture
        .scene
        .borrow_mut()
        .set_transform(node, right)
        .unwrap();
    render_counted(&mut renderer, &gpu).expect("renders");

    // When
    set(&fixture, node, red, true);
    let (pixels, work) = render_counted(&mut renderer, &gpu).expect("renders");

    // Then: 20 pixels a unit, so x = 2 is at pixel 80 + 40
    assert_eq!((work.buffer_bytes, work.draw_calls), (MESH_BYTES, 2));
    let ([x, _], count) = centroid(&pixels, RED);
    assert!(count > 20 && (x - 120.0).abs() <= 1.0, "{count} at {x}");
}

#[test]
fn gpu_a_mesh_shown_again_unchanged_is_drawn_and_nothing_is_written() {
    // Given
    let gpu = vapors_gpu_harness::gpu();
    let (fixture, mut renderer, node) = drawn_once(&gpu);
    let red = Colour::srgb(1.0, 0.0, 0.0).unwrap();
    set(&fixture, node, red, false);
    render_counted(&mut renderer, &gpu).expect("renders");

    // When
    set(&fixture, node, red, true);
    let (pixels, work) = render_counted(&mut renderer, &gpu).expect("renders");

    // Then
    assert_eq!((work.buffer_bytes, work.draw_calls), (0, 2));
    assert!(centroid(&pixels, RED).1 > 20);
}
