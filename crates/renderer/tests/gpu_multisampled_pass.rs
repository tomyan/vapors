//! GPU tier: a pass at 4 samples (`docs/plans/flat-scene-f7.md`, Q3)
//! draws into a multisampled view and resolves into the target's at its
//! end, storing the multisampled colour so a later pass on the view in
//! the same frame (here after a flush) draws over what the first drew.

#[path = "support/frame.rs"]
mod frame;

use frame::{BACKGROUND, pixel, quad, rgba, square, target};
use vapors_gpu_harness::{Gpu, gpu, read_rgba8};
use vapors_renderer::{FrameEncoder, PassFormat, QuadPipeline, Samples, Target, clear_target};

const SIDE: u32 = 16;
const RED: [u8; 4] = [255, 0, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];

/// A 4x colour view of `format`, `SIDE` square.
fn multisampled(gpu: &Gpu, format: wgpu::TextureFormat) -> wgpu::TextureView {
    let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("test multisampled"),
        size: wgpu::Extent3d {
            width: SIDE,
            height: SIDE,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 4,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

/// The pixels of a frame at `samples`: cleared, a red quad, a flush that
/// ends the pass, then a blue quad in a second pass on the same view.
fn two_passes(samples: Samples) -> Vec<u8> {
    let gpu = gpu();
    let (texture, view) = square(&gpu, SIDE);
    let format = texture.format();
    let mut quads = QuadPipeline::new(&gpu.device, PassFormat { format, samples });
    let four = (samples == Samples::Four).then(|| multisampled(&gpu, format));
    let encoder = FrameEncoder::new(&gpu.device);
    let at = Target {
        multisampled: four.as_ref(),
        ..target(&gpu, &encoder, &view, SIDE)
    };
    clear_target(&at, rgba(BACKGROUND));
    quads.draw(&at, &[quad([0.0, 0.0, 8.0, 16.0], RED)]);
    encoder.flush(&gpu.queue);
    quads.draw(&at, &[quad([8.0, 0.0, 16.0, 16.0], BLUE)]);
    encoder.submit(&gpu.queue).expect("no refusal");
    read_rgba8(&gpu.device, &gpu.queue, &texture)
}

#[test]
fn gpu_a_second_pass_at_4_samples_draws_over_the_first_as_at_1() {
    // When
    let four = two_passes(Samples::Four);

    // Then: the first pass's quad survived the second, and the frame is
    // the 1-sample frame (quads on whole pixels cover every sample)
    assert_eq!(pixel(&four, SIDE, 3, 8), RED);
    assert_eq!(pixel(&four, SIDE, 12, 8), BLUE);
    assert!(four == two_passes(Samples::One), "4 samples differ from 1");
}
