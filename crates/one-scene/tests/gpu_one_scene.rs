//! GPU tier (lavapipe): the example driven by its script for the first
//! 940 frames of the run, past the first click, the first hide and the
//! first night. Every frame its scheduled classes reach the meshes
//! (`script::confirm`), and the scene's work is what the changes cost:
//! one 80-byte uniform per shown cube a frame (the group turns), none for
//! a hidden one, 80 more for the ground in the frame the night token
//! changes, no geometry after the first frame, one draw fewer while a
//! cube is hidden. Counts, never times.

use vapors_app::GpuWork;
use vapors_gpu_harness::Gpu;
use vapors_one_scene::OneScene;
use vapors_one_scene::schedule::{CUBES, expected, time_of};
use vapors_one_scene::script::{confirm, drive};
use vapors_platform::{App, Frame, Instant, Next};
use vapors_renderer::clear;
use vapors_scene::Color;

/// A mesh node's uniform: its world matrix and colour.
const MESH_BYTES: u64 = 80;

/// Frame `index` of the run at the frame clock's time, into a small
/// target; what it asked for next and the scene's work.
fn frame(scene: &mut OneScene, gpu: &Gpu, start: Instant, index: usize) -> (Next, GpuWork) {
    let texture = clear(&gpu.device, &gpu.queue, 320, 200, Color::default());
    let next = scene.frame(&Frame {
        adapter: &gpu.adapter,
        device: &gpu.device,
        queue: &gpu.queue,
        texture: &texture,
        index: index as u64,
        scale_factor: 1.0,
        now: start + time_of(index),
    });
    (next, scene.frame_counts().surfaces)
}

/// The uniform bytes frame `index` writes after the first: each shown
/// cube, and the ground when the night token changed.
fn bytes_due(index: usize) -> u64 {
    let (now, before) = (expected(index), expected(index - 1));
    let cubes = CUBES as u64 - u64::from(now.gone);
    MESH_BYTES * (cubes + u64::from(now.night != before.night))
}

#[test]
fn gpu_the_scripted_changes_reach_the_meshes_at_the_cost_the_schedule_predicts() {
    // Given
    let gpu = vapors_gpu_harness::gpu();
    let mut scene = OneScene::new().expect("mounts");
    let start = Instant::now();
    let (next, first) = frame(&mut scene, &gpu, start, 0);
    assert_eq!(next, Next::Continue, "frame 0 drawn: {first:?}");
    confirm(&scene, 0).expect("frame 0");
    let composition = first.draw_calls - 4;

    for index in 1..=940 {
        // When
        drive(&mut scene, index).expect("driven");
        let (next, work) = frame(&mut scene, &gpu, start, index);

        // Then
        confirm(&scene, index).expect("confirmed");
        assert_eq!(next, Next::Continue, "frame {index} asks for the next");
        assert_eq!(work.buffer_bytes, bytes_due(index), "frame {index}");
        let shown = 1 + CUBES as u64 - u64::from(expected(index).gone);
        assert_eq!(work.draw_calls, composition + shown, "frame {index}");
    }
}
