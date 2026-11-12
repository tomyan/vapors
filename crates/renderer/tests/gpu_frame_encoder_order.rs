//! GPU tier: a pipeline draws for one `FrameEncoder` at a time. A draw
//! for an older encoder after a newer one, or for a newer one while the
//! older is still open, would overwrite the older's bytes, and panics;
//! a newer one after the older is dropped unsubmitted is fine.

#[path = "support/frame.rs"]
mod frame;

use frame::{quad, square, target};
use vapors_gpu_harness::{Gpu, gpu};
use vapors_renderer::{FrameEncoder, QuadPipeline};

const SIDE: u32 = 4;
const RED: [u8; 4] = [255, 0, 0, 255];

/// Draws one quad through `pipeline` for `encoder`.
fn draw_one(gpu: &Gpu, pipeline: &mut QuadPipeline, encoder: &FrameEncoder) {
    let (_texture, view) = square(gpu, SIDE);
    let target = target(gpu, encoder, &view, SIDE);
    pipeline.draw(&target, &[quad([0.0, 0.0, 1.0, 1.0], RED)]);
}

fn pipeline(gpu: &Gpu) -> QuadPipeline {
    QuadPipeline::new(&gpu.device, wgpu::TextureFormat::Rgba8Unorm)
}

#[test]
#[should_panic(expected = "an older encoder after a newer one")]
fn gpu_a_draw_for_an_older_encoder_after_a_newer_one_panics() {
    // Given
    let gpu = gpu();
    let mut pipeline = pipeline(&gpu);
    let (older, newer) = (
        FrameEncoder::new(&gpu.device),
        FrameEncoder::new(&gpu.device),
    );
    draw_one(&gpu, &mut pipeline, &newer);

    // When
    draw_one(&gpu, &mut pipeline, &older);
}

#[test]
#[should_panic(expected = "which this pipeline drew for, is still open")]
fn gpu_a_draw_for_a_newer_encoder_while_the_older_is_open_panics() {
    // Given
    let gpu = gpu();
    let mut pipeline = pipeline(&gpu);
    let older = FrameEncoder::new(&gpu.device);
    draw_one(&gpu, &mut pipeline, &older);
    let newer = FrameEncoder::new(&gpu.device);

    // When
    draw_one(&gpu, &mut pipeline, &newer);
}

#[test]
fn gpu_a_draw_for_a_newer_encoder_after_the_older_is_dropped_is_allowed() {
    // Given
    let gpu = gpu();
    let mut pipeline = pipeline(&gpu);
    let older = FrameEncoder::new(&gpu.device);
    draw_one(&gpu, &mut pipeline, &older);
    drop(older);
    let newer = FrameEncoder::new(&gpu.device);

    // When
    draw_one(&gpu, &mut pipeline, &newer);

    // Then
    newer.submit(&gpu.queue).expect("no draw refused");
}
