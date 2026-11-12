//! GPU tier: the page's scene drawn by its `SceneRenderer` in an
//! `ElementsApp`. Its draws are counted in `FrameCounts::surfaces`
//! (`docs/plans/3d-2.md` decision 4), and a transform or colour signal
//! change renders the surface again, writing one mesh's uniform and no
//! geometry.

mod support;

use support::{Rig, at};
use vapors_app::{ElementsApp, GpuWork};
use vapors_gpu_harness::{Gpu, read_rgba8};
use vapors_platform::{App, Frame, Instant, Next};
use vapors_renderer::clear;
use vapors_scene::Color;
use vapors_scene3d::{Colour, Transform, Vec3};

/// The target's side in device pixels: the page's surface fills it.
const SIZE: u32 = 64;
/// A mesh node's uniform: its world matrix and colour, 20 floats.
const MESH_BYTES: u64 = 80;
/// The camera's view-projection, 16 floats.
const CAMERA_BYTES: u64 = 64;
/// The `{if}` mesh's default colour, at 1 unit right of centre.
const BLACK: [u8; 4] = [0, 0, 0, 255];
const RED: [u8; 4] = [255, 0, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];

/// One app frame into a fresh target: what it asked for next, the
/// surfaces' counted work and the frame's pixels.
fn frame(app: &mut ElementsApp, gpu: &Gpu) -> (Next, GpuWork, Vec<u8>) {
    let texture = clear(&gpu.device, &gpu.queue, SIZE, SIZE, Color::default());
    let next = app.frame(&Frame {
        adapter: &gpu.adapter,
        device: &gpu.device,
        queue: &gpu.queue,
        texture: &texture,
        index: 0,
        scale_factor: 1.0,
        now: Instant::now(),
    });
    let pixels = read_rgba8(&gpu.device, &gpu.queue, &texture);
    (next, app.frame_counts().surfaces, pixels)
}

/// The pixel at `(x, y)`.
fn pixel(pixels: &[u8], x: u32, y: u32) -> [u8; 4] {
    let at = ((y * SIZE + x) * 4) as usize;
    pixels[at..at + 4].try_into().expect("four bytes")
}

/// The page drawn, then an idle frame: the rig, and the idle frame's
/// surface work (its composition alone).
fn settled(gpu: &Gpu) -> (Rig, GpuWork) {
    let mut rig = Rig::new();
    frame(&mut rig.app, gpu);
    let (next, idle, _) = frame(&mut rig.app, gpu);
    assert_eq!(next, Next::Wait, "an idle page waits");
    (rig, idle)
}

#[test]
fn gpu_the_scene_draws_its_mesh_and_frame_counts_hold_the_draw() {
    // Given
    let gpu = vapors_gpu_harness::gpu();
    let mut rig = Rig::new();

    // When
    let (_, first, pixels) = frame(&mut rig.app, &gpu);

    // Then: one composition draw and one mesh draw; 16 pixels a unit
    assert_eq!(first.draw_calls, 2);
    assert_eq!(pixel(&pixels, 32, 32), RED);
    assert_ne!(pixel(&pixels, 12, 32), RED);
}

#[test]
fn gpu_a_transform_signal_change_writes_one_uniform_and_uploads_no_geometry() {
    // Given
    let gpu = vapors_gpu_harness::gpu();
    let (mut rig, idle) = settled(&gpu);

    // When: the square moves 1 unit left
    rig.signals.left.set(at(-1.0));
    let (_, moved, pixels) = frame(&mut rig.app, &gpu);

    // Then
    assert_eq!(moved.buffer_bytes, idle.buffer_bytes + MESH_BYTES);
    assert_eq!(moved.texture_bytes, 0);
    assert_eq!(moved.draw_calls, idle.draw_calls + 1);
    assert_eq!(pixel(&pixels, 16, 32), RED);
    assert_ne!(pixel(&pixels, 32, 32), RED);
}

#[test]
fn gpu_a_colour_signal_change_writes_one_uniform_and_recolours_the_mesh() {
    // Given
    let gpu = vapors_gpu_harness::gpu();
    let (mut rig, idle) = settled(&gpu);

    // When
    rig.signals
        .left_colour
        .set(Colour::srgb(0.0, 0.0, 1.0).unwrap());
    let (_, recoloured, pixels) = frame(&mut rig.app, &gpu);

    // Then
    assert_eq!(recoloured.buffer_bytes, idle.buffer_bytes + MESH_BYTES);
    assert_eq!(pixel(&pixels, 32, 32), BLUE);
}

#[test]
fn gpu_a_camera_transform_signal_change_writes_the_camera_uniform_alone() {
    // Given
    let gpu = vapors_gpu_harness::gpu();
    let (mut rig, idle) = settled(&gpu);

    // When: the camera moves 1 unit right, so the square shows 1 unit left
    rig.signals
        .back
        .set(Transform::from_translation(Vec3::new(1.0, 0.0, 5.0)).unwrap());
    let (_, moved, pixels) = frame(&mut rig.app, &gpu);

    // Then
    assert_eq!(moved.buffer_bytes, idle.buffer_bytes + CAMERA_BYTES);
    assert_eq!(pixel(&pixels, 16, 32), RED);
    assert_ne!(pixel(&pixels, 32, 32), RED);
}

#[test]
fn gpu_a_mesh_hidden_by_its_if_is_not_drawn_and_shown_again_is_drawn_once() {
    // Given: the if-mesh shown and drawn, 1 unit right of centre
    let gpu = vapors_gpu_harness::gpu();
    let (mut rig, idle) = settled(&gpu);
    rig.signals.shown.set(true);
    let (_, _, shown) = frame(&mut rig.app, &gpu);
    assert_eq!(pixel(&shown, 48, 32), BLACK);

    // When
    rig.signals.shown.set(false);
    let (_, _, hidden) = frame(&mut rig.app, &gpu);
    rig.signals.shown.set(true);
    let (_, again, _) = frame(&mut rig.app, &gpu);

    // Then
    assert_ne!(pixel(&hidden, 48, 32), BLACK);
    assert_eq!(
        again.draw_calls,
        idle.draw_calls + 2,
        "the mesh and one if-mesh"
    );
    assert_eq!(rig.scene.borrow().len(), 5);
}
