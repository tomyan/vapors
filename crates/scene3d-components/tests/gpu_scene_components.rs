//! GPU tier: the page's scene drawn by its `SceneRenderer` in an
//! `ElementsApp`. Its draws are counted in `FrameCounts::surfaces`
//! (`docs/plans/3d-2.md` decision 4). A transform signal change, or a
//! class or token change to a mesh's computed `color`
//! (`docs/plans/css-for-3d.md` C3), renders the surface again, writing
//! one uniform per mesh it reaches and no geometry.

mod support;

use support::{Rig, at};
use vapors_app::{ElementsApp, GpuWork};
use vapors_dom::Dom;
use vapors_gpu_harness::{Gpu, read_rgba8};
use vapors_platform::{App, Frame, Instant, Next};
use vapors_renderer::clear;
use vapors_scene::Color;
use vapors_scene3d::{Transform, Vec3};

/// The target's side in device pixels: the page's surface fills it.
const SIZE: u32 = 64;
/// A mesh node's uniform: its world matrix and colour, 20 floats.
const MESH_BYTES: u64 = 80;
/// The camera's view-projection, 16 floats.
const CAMERA_BYTES: u64 = 64;
/// The `--tint` token's colour, which both meshes take.
const RED: [u8; 4] = [255, 0, 0, 255];
/// The token's colour under `html.night`.
const YELLOW: [u8; 4] = [255, 255, 0, 255];
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
fn gpu_an_idle_frame_writes_nothing() {
    // Given
    let gpu = vapors_gpu_harness::gpu();

    // When
    let (_, idle) = settled(&gpu);

    // Then
    assert_eq!(idle.buffer_bytes, 0);
}

#[test]
fn gpu_a_class_toggle_recolours_one_mesh_writing_one_uniform_and_no_geometry() {
    // Given: the {if} mesh shown too, 1 unit right of centre
    let gpu = vapors_gpu_harness::gpu();
    let (mut rig, idle) = settled(&gpu);
    rig.signals.shown.set(true);
    frame(&mut rig.app, &gpu);

    // When
    rig.signals.blue.set(true);
    let (_, recoloured, pixels) = frame(&mut rig.app, &gpu);

    // Then
    assert_eq!(recoloured.buffer_bytes, idle.buffer_bytes + MESH_BYTES);
    assert_eq!(recoloured.texture_bytes, 0);
    assert_eq!(pixel(&pixels, 32, 32), BLUE);
    assert_eq!(pixel(&pixels, 48, 32), RED);
}

#[test]
fn gpu_a_token_change_recolours_every_mesh_that_reads_it_one_uniform_each() {
    // Given: `.left` reads `--tint`, the {if} mesh inherits the body's
    // `color`, which reads it too
    let gpu = vapors_gpu_harness::gpu();
    let (mut rig, idle) = settled(&gpu);
    rig.signals.shown.set(true);
    frame(&mut rig.app, &gpu);
    let root = rig.doc.borrow().root();

    // When
    rig.doc.borrow_mut().set_attribute(root, "class", "night");
    let (_, recoloured, pixels) = frame(&mut rig.app, &gpu);

    // Then
    assert_eq!(recoloured.buffer_bytes, idle.buffer_bytes + 2 * MESH_BYTES);
    assert_eq!(pixel(&pixels, 32, 32), YELLOW);
    assert_eq!(pixel(&pixels, 48, 32), YELLOW);
}

#[test]
fn gpu_visibility_hidden_drops_one_draw_and_writes_nothing() {
    // Given
    let gpu = vapors_gpu_harness::gpu();
    let (mut rig, idle) = settled(&gpu);

    // When
    rig.signals.gone.set(true);
    let (_, hidden, pixels) = frame(&mut rig.app, &gpu);

    // Then: the composition's draw alone
    assert_eq!(hidden.buffer_bytes, idle.buffer_bytes);
    assert_eq!(hidden.draw_calls, idle.draw_calls);
    assert_ne!(pixel(&pixels, 32, 32), RED);
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
    assert_eq!(pixel(&shown, 48, 32), RED);

    // When
    rig.signals.shown.set(false);
    let (_, _, hidden) = frame(&mut rig.app, &gpu);
    rig.signals.shown.set(true);
    let (_, again, _) = frame(&mut rig.app, &gpu);

    // Then
    assert_ne!(pixel(&hidden, 48, 32), RED);
    assert_eq!(
        again.draw_calls,
        idle.draw_calls + 2,
        "the mesh and one if-mesh"
    );
    assert_eq!(rig.scene.borrow().len(), 5);
}
