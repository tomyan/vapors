//! GPU tier: a draw whose bytes need a buffer over the device's
//! `max_buffer_size` is refused with the pipeline, the size and the
//! limit named, and is not recorded; the encoder's submit returns it.

#[path = "support/frame.rs"]
mod frame;

use frame::{BACKGROUND, pixel, quad, square, target};
use vapors_gpu_harness::{gpu, read_rgba8};
use vapors_renderer::{ArenaFull, FrameEncoder, QuadPipeline};

/// The lowered `max_buffer_size`: twice the arena's first buffer.
const LIMIT: u64 = 128 * 1024;
/// A quad's instance bytes.
const QUAD_BYTES: u64 = 128;
const SIDE: u32 = 4;

#[test]
fn gpu_a_draw_over_the_devices_max_buffer_size_is_refused_and_not_recorded() {
    // Given: one draw of more quads than LIMIT bytes hold
    let gpu = gpu().with_max_buffer_size(LIMIT);
    let (texture, view) = square(&gpu, SIDE);
    let mut pipeline = QuadPipeline::new(&gpu.device, texture.format());
    let count = LIMIT / QUAD_BYTES + 1;
    let quads: Vec<_> = (0..count)
        .map(|_| quad([0.0, 0.0, 4.0, 4.0], [255, 0, 0, 255]))
        .collect();
    let encoder = FrameEncoder::new(&gpu.device);

    // When
    pipeline.draw(&target(&gpu, &encoder, &view, SIDE), &quads);
    let submitted = encoder.submit(&gpu.queue);

    // Then
    let refused = ArenaFull {
        pipeline: "quads",
        size: count * QUAD_BYTES,
        limit: LIMIT,
    };
    let pixels = read_rgba8(&gpu.device, &gpu.queue, &texture);
    assert_eq!(submitted, Err(refused), "the refusal");
    assert_eq!(pixel(&pixels, SIDE, 1, 1), BACKGROUND, "nothing drawn");
}
