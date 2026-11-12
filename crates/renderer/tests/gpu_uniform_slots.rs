//! GPU tier: a pipeline's viewport uniform is kept per size seen
//! (`uniform_slots.rs`): written when a size first appears, bound by
//! its offset after; a frame drawing more sizes than the buffer holds
//! moves to a larger one, with a bind group of its own; and each draw
//! reads its own size.

#[path = "support/frame.rs"]
mod frame;

use frame::{BACKGROUND, pixel, quad, square, target};
use vapors_gpu_harness::{Gpu, gpu, read_rgba8};
use vapors_renderer::{FrameEncoder, GpuWork, QuadPipeline};

const RED: [u8; 4] = [255, 0, 0, 255];
/// Slots in a new uniform buffer (`FIRST_SLOTS` in `uniform_slots.rs`).
const FIRST_SLOTS: u32 = 16;

/// One frame drawing one quad on a square of each of `sides`: the
/// frame's work.
fn frame_at(gpu: &Gpu, pipeline: &mut QuadPipeline, sides: &[u32]) -> GpuWork {
    let encoder = FrameEncoder::new(&gpu.device);
    let targets: Vec<_> = sides
        .iter()
        .map(|&side| (side, square(gpu, side)))
        .collect();
    for (side, (_, view)) in &targets {
        let target = target(gpu, &encoder, view, *side);
        pipeline.draw(&target, &[quad([0.0, 0.0, 1.0, 1.0], RED)]);
    }
    encoder.submit(&gpu.queue).expect("no draw refused");
    pipeline.take_work()
}

#[test]
fn gpu_a_viewport_seen_before_is_neither_written_nor_bound_again() {
    // Given
    let gpu = gpu();
    let mut pipeline = QuadPipeline::new(&gpu.device, wgpu::TextureFormat::Rgba8Unorm);

    // When: squares of 4 then 8, then each again
    let first = frame_at(&gpu, &mut pipeline, &[4]);
    let second = frame_at(&gpu, &mut pipeline, &[8]);
    let again = [
        frame_at(&gpu, &mut pipeline, &[4]),
        frame_at(&gpu, &mut pipeline, &[8]),
    ];

    // Then
    assert_eq!((first.uniform_writes, first.bind_groups_made), (1, 1));
    assert_eq!((second.uniform_writes, second.bind_groups_made), (1, 0));
    for work in again {
        assert_eq!((work.uniform_writes, work.bind_groups_made), (0, 0));
    }
}

#[test]
fn gpu_a_frame_with_more_viewports_than_slots_moves_to_a_larger_buffer_once() {
    // Given
    let gpu = gpu();
    let mut pipeline = QuadPipeline::new(&gpu.device, wgpu::TextureFormat::Rgba8Unorm);
    let sides: Vec<u32> = (1..=FIRST_SLOTS + 1).collect();

    // When: one more size than the first buffer holds, then the last size again
    let crowded = frame_at(&gpu, &mut pipeline, &sides);
    let last = frame_at(&gpu, &mut pipeline, &[FIRST_SLOTS + 1]);

    // Then: every size written, a bind group per buffer; the last size kept in the new buffer
    assert_eq!(crowded.uniform_writes, u64::from(FIRST_SLOTS + 1));
    assert_eq!(crowded.bind_groups_made, 2);
    assert_eq!((last.uniform_writes, last.bind_groups_made), (0, 0));
}

#[test]
fn gpu_each_draw_in_a_frame_reads_its_own_viewport() {
    // Given: a quad over the top-left quarter of a 4 square and of an 8 square
    let gpu = gpu();
    let mut pipeline = QuadPipeline::new(&gpu.device, wgpu::TextureFormat::Rgba8Unorm);
    let encoder = FrameEncoder::new(&gpu.device);
    let (small, small_view) = square(&gpu, 4);
    let (large, large_view) = square(&gpu, 8);

    // When
    pipeline.draw(
        &target(&gpu, &encoder, &small_view, 4),
        &[quad([0.0, 0.0, 2.0, 2.0], RED)],
    );
    pipeline.draw(
        &target(&gpu, &encoder, &large_view, 8),
        &[quad([0.0, 0.0, 4.0, 4.0], RED)],
    );
    encoder.submit(&gpu.queue).expect("no draw refused");

    // Then: each quad ends at its own square's midpoint
    let small = read_rgba8(&gpu.device, &gpu.queue, &small);
    let large = read_rgba8(&gpu.device, &gpu.queue, &large);
    assert_eq!(pixel(&small, 4, 1, 1), RED);
    assert_eq!(pixel(&small, 4, 2, 2), BACKGROUND);
    assert_eq!(pixel(&large, 8, 3, 3), RED);
    assert_eq!(pixel(&large, 8, 4, 4), BACKGROUND);
}

#[test]
fn gpu_a_draw_after_the_buffer_grows_reads_its_viewport_from_the_new_buffer() {
    // Given: one more size than the first buffer holds
    let gpu = gpu();
    let mut pipeline = QuadPipeline::new(&gpu.device, wgpu::TextureFormat::Rgba8Unorm);
    let encoder = FrameEncoder::new(&gpu.device);
    let squares: Vec<_> = (1..=FIRST_SLOTS + 1)
        .map(|side| (side, square(&gpu, side)))
        .collect();

    // When: a quad over each square's top-left pixel
    for (side, (_, view)) in &squares {
        let target = target(&gpu, &encoder, view, *side);
        pipeline.draw(&target, &[quad([0.0, 0.0, 1.0, 1.0], RED)]);
    }
    encoder.submit(&gpu.queue).expect("no draw refused");

    // Then: the last quad, drawn after the move, covers one pixel of its square
    let (side, (last, _)) = squares.last().expect("one square per size");
    let rgba = read_rgba8(&gpu.device, &gpu.queue, last);
    assert_eq!(pixel(&rgba, *side, 0, 0), RED);
    assert_eq!(pixel(&rgba, *side, 1, 1), BACKGROUND);
}
