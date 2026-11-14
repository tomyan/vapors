//! GPU tier (lavapipe): the example driven by its script for the first
//! 1,880 frames of the run, past the first click, the first hide, and
//! the night coming and going. Every frame its scheduled classes reach the meshes
//! (`script::confirm`), and the scene's work is what the changes cost:
//! one 80-byte uniform per shown cube a frame (the group turns), none for
//! a hidden one, 80 more for the ground in the frame the night token
//! changes, no geometry after the first frame, one draw fewer while a
//! cube is hidden. Counts, never times.

use vapors_app::GpuWork;
use vapors_gpu_harness::Gpu;
use vapors_one_scene::OneScene;
use vapors_one_scene::confirm::confirm;
use vapors_one_scene::schedule::{CUBES, expected, time_of};
use vapors_one_scene::script::drive;
use vapors_platform::{App, Frame, Instant, Next};
use vapors_renderer::clear;
use vapors_scene::Color;

/// A mesh node's uniform: its world matrix and colour.
const MESH_BYTES: u64 = 80;

/// The cube (24 vertices of 16 bytes, position and shade; 36 indices of
/// 4) and the ground (4 vertices, 6 indices).
const GEOMETRY_BYTES: u64 = 24 * 16 + 36 * 4 + 4 * 16 + 6 * 4;

/// Frame 0's surface work: the geometry, four mesh uniforms, the
/// camera's 64 bytes, and 112 more outside the scene renderer (measured
/// 2026-11-14: 808 in all before the cube had its own vertices a face; not
/// derived).
const FIRST_FRAME_BYTES: u64 = GEOMETRY_BYTES + 4 * MESH_BYTES + 64 + 112;

/// The surface's own draw in the composition, beside the meshes'.
const COMPOSITION_DRAWS: u64 = 1;

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
    let mut scene = OneScene::new(None).expect("mounts");
    let start = Instant::now();
    let (next, first) = frame(&mut scene, &gpu, start, 0);
    assert_eq!(next, Next::Continue, "frame 0 drawn: {first:?}");
    confirm(&scene, 0).expect("frame 0");
    assert_eq!(first.buffer_bytes, FIRST_FRAME_BYTES, "frame 0: {first:?}");
    assert_eq!(
        first.draw_calls,
        COMPOSITION_DRAWS + 4,
        "frame 0: {first:?}"
    );
    let composition = COMPOSITION_DRAWS;

    for index in 1..=1880 {
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

#[test]
fn gpu_a_limited_run_stops_its_clock_and_goes_idle_after_its_frames() {
    // Given: the web page's `ticks=5`
    let gpu = vapors_gpu_harness::gpu();
    let mut scene = OneScene::new(Some(5)).expect("mounts");
    let start = Instant::now();

    // When
    let nexts: Vec<Next> = (0..8)
        .map(|index| frame(&mut scene, &gpu, start, index).0)
        .collect();
    let (_, idle) = frame(&mut scene, &gpu, start, 8);

    // Then: five frames tick, the fifth stops the surface; a frame drawn
    // after that (the loop would not ask for one) writes nothing
    assert_eq!(&nexts[..4], &[Next::Continue; 4]);
    assert_eq!(nexts.last(), Some(&Next::Wait), "{nexts:?}");
    assert_eq!(idle.buffer_bytes, 0);
}
