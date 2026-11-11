//! GPU tier: draws recorded into one `FrameEncoder` and submitted
//! together each read their own data: instances past the pipeline's
//! first buffer, and viewports that differ between targets.

#[path = "support/frame.rs"]
mod frame;

use frame::{BACKGROUND, pixel, quad, square, target};
use vapors_gpu_harness::{gpu, read_rgba8};
use vapors_renderer::{FrameEncoder, QuadPipeline};

/// More draws than the first 64 KiB instance buffer holds (a quad is
/// 128 bytes: 512 draws), so the arena grows mid-frame.
const DRAWS: u32 = 600;
const SIDE: u32 = 32;

/// Draw `index`'s colour: its own red, so a draw that read another
/// draw's data shows.
fn colour(index: u32) -> [u8; 4] {
    let red = u8::try_from(index % 250).expect("under 250") + 5;
    [red, 0, 0, 255]
}

/// The one-pixel square of draw `index`, at `(index % SIDE, index / SIDE)`.
#[allow(clippy::cast_precision_loss)] // small pixel positions
fn cell(index: u32) -> [f32; 4] {
    let (x, y) = ((index % SIDE) as f32, (index / SIDE) as f32);
    [x, y, x + 1.0, y + 1.0]
}

#[test]
fn gpu_every_draw_of_one_encoder_reads_its_own_instances() {
    // Given
    let gpu = gpu();
    let (texture, view) = square(&gpu, SIDE);
    let mut pipeline = QuadPipeline::new(&gpu.device, texture.format());
    let encoder = FrameEncoder::new(&gpu.device);

    // When: one draw call per quad, one submit
    for index in 0..DRAWS {
        let target = target(&gpu, &encoder, &view, SIDE);
        pipeline.draw(&target, &[quad(cell(index), colour(index))]);
    }
    encoder.submit(&gpu.queue).expect("no draw refused");

    // Then
    let pixels = read_rgba8(&gpu.device, &gpu.queue, &texture);
    let wrong: Vec<u32> = (0..DRAWS)
        .filter(|&i| pixel(&pixels, SIDE, i % SIDE, i / SIDE) != colour(i))
        .collect();
    assert!(wrong.is_empty(), "draws that read other data: {wrong:?}");
}

#[test]
fn gpu_draws_of_one_encoder_on_targets_of_two_sizes_each_read_their_own_viewport() {
    // Given: an 8 px and a 16 px target, each with a quad over its
    // bottom-right quarter; read through the other's viewport, the
    // small one's quad lands on (2..4, 2..4) and the large one's off it
    const RED: [u8; 4] = [255, 0, 0, 255];
    const BLUE: [u8; 4] = [0, 0, 255, 255];
    let gpu = gpu();
    let ((small, small_view), (large, large_view)) = (square(&gpu, 8), square(&gpu, 16));
    let mut pipeline = QuadPipeline::new(&gpu.device, small.format());
    let encoder = FrameEncoder::new(&gpu.device);

    // When
    let on_small = target(&gpu, &encoder, &small_view, 8);
    pipeline.draw(&on_small, &[quad([4.0, 4.0, 8.0, 8.0], RED)]);
    let on_large = target(&gpu, &encoder, &large_view, 16);
    pipeline.draw(&on_large, &[quad([8.0, 8.0, 16.0, 16.0], BLUE)]);
    encoder.submit(&gpu.queue).expect("no draw refused");

    // Then
    let small = read_rgba8(&gpu.device, &gpu.queue, &small);
    let large = read_rgba8(&gpu.device, &gpu.queue, &large);
    let seen = [
        pixel(&small, 8, 6, 6),
        pixel(&small, 8, 3, 3),
        pixel(&large, 16, 12, 12),
    ];
    assert_eq!(
        seen,
        [RED, BACKGROUND, BLUE],
        "small inside, small outside, large inside"
    );
}
