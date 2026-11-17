//! GPU tier: the frame's depth attachment (flat-scene F8a). A pass
//! refuses one of the wrong samples or size on the CPU, naming both
//! (#493 review, M1); and a pass without depth after one with it on the
//! same view begins its own pass, which keeps what the first drew (L4).

#[path = "support/frame.rs"]
mod frame;

use frame::{BACKGROUND, pixel, quad, rgba, square, target};
use vapors_gpu_harness::{gpu, read_rgba8};
use vapors_renderer::{
    DepthAttachment, FrameEncoder, PassFormat, QuadPipeline, Samples, Target, clear_target,
};
use vapors_scene::Size;

const SIDE: u32 = 16;
const RED: [u8; 4] = [255, 0, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];

/// A red quad drawn on a 1-sample `SIDE` square target whose depth
/// attachment is `size` at `samples`.
fn draw_with(size: Size, samples: Samples) {
    let gpu = gpu();
    let (texture, view) = square(&gpu, SIDE);
    let pass = PassFormat {
        format: texture.format(),
        samples: Samples::One,
    };
    let mut quads = QuadPipeline::new(&gpu.device, pass);
    let depth = DepthAttachment::new(&gpu.device, size, samples);
    let encoder = FrameEncoder::new(&gpu.device);
    let at = Target {
        depth: Some(&depth),
        ..target(&gpu, &encoder, &view, SIDE)
    };
    quads.draw(&at, &[quad([0.0, 0.0, 8.0, 8.0], RED)]);
}

fn side(side: u32) -> Size {
    Size {
        width: side,
        height: side,
    }
}

#[test]
#[should_panic(
    expected = "a depth attachment of 4 samples and Size { width: 16, height: 16 } drawn with a target of 1 samples"
)]
fn gpu_a_depth_attachment_of_other_samples_is_refused_naming_both() {
    draw_with(side(SIDE), Samples::Four);
}

#[test]
#[should_panic(
    expected = "a depth attachment of 1 samples and Size { width: 8, height: 8 } drawn with a target of 1 samples and Size { width: 16, height: 16 }"
)]
fn gpu_a_depth_attachment_of_another_size_is_refused_naming_both() {
    draw_with(side(8), Samples::One);
}

#[test]
fn gpu_a_pass_without_depth_after_one_with_it_on_the_view_keeps_what_it_drew() {
    // Given
    let gpu = gpu();
    let (texture, view) = square(&gpu, SIDE);
    let pass = PassFormat {
        format: texture.format(),
        samples: Samples::One,
    };
    let mut quads = QuadPipeline::new(&gpu.device, pass);
    let depth = DepthAttachment::new(&gpu.device, side(SIDE), Samples::One);
    let encoder = FrameEncoder::new(&gpu.device);
    let flat = target(&gpu, &encoder, &view, SIDE);
    let deep = Target {
        depth: Some(&depth),
        ..flat
    };

    // When: a red quad with depth, then a blue one without
    clear_target(&deep, rgba(BACKGROUND));
    quads.draw(&deep, &[quad([0.0, 0.0, 8.0, 16.0], RED)]);
    quads.draw(&flat, &[quad([8.0, 0.0, 16.0, 16.0], BLUE)]);
    let passes = encoder.render_passes();
    encoder.submit(&gpu.queue).expect("no refusal");

    // Then: two passes, and both quads
    let pixels = read_rgba8(&gpu.device, &gpu.queue, &texture);
    assert_eq!(passes, 2);
    assert_eq!(
        (pixel(&pixels, SIDE, 2, 8), pixel(&pixels, SIDE, 12, 8)),
        (RED, BLUE)
    );
}
