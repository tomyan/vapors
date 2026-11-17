//! GPU tier: a pass at 4 samples (`docs/plans/flat-scene-f7.md`, Q3)
//! draws into a multisampled view and resolves into the target's at its
//! end, storing the multisampled colour so a later pass on the view in
//! the same frame (here after a flush) draws over what the first drew; a
//! 1-sample pass on the same view is a pass of its own; a pipeline drawn
//! into a target of another sample count is refused by name; and a pass
//! that would load a multisampled view not cleared in the frame is
//! refused.

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

/// The pixels of a frame at `samples` on `gpu`: cleared, a red quad, a
/// flush that ends the pass, then a blue quad in a second pass on the
/// same view.
fn two_passes(gpu: &Gpu, samples: Samples) -> Vec<u8> {
    let (texture, view) = square(gpu, SIDE);
    let format = texture.format();
    let mut quads = QuadPipeline::new(&gpu.device, PassFormat { format, samples });
    let four = (samples == Samples::Four).then(|| multisampled(gpu, format));
    let encoder = FrameEncoder::new(&gpu.device);
    let at = Target {
        multisampled: four.as_ref(),
        ..target(gpu, &encoder, &view, SIDE)
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
    let gpu = gpu();
    let four = two_passes(&gpu, Samples::Four);

    // Then: the first pass's quad survived the second, and the frame is
    // the 1-sample frame (quads on whole pixels cover every sample)
    assert_eq!(pixel(&four, SIDE, 3, 8), RED);
    assert_eq!(pixel(&four, SIDE, 12, 8), BLUE);
    assert!(
        four == two_passes(&gpu, Samples::One),
        "4 samples differ from 1"
    );
}

/// A quad pipeline at `samples` for `format` on `gpu`.
fn quads_at(gpu: &Gpu, format: wgpu::TextureFormat, samples: Samples) -> QuadPipeline {
    QuadPipeline::new(&gpu.device, PassFormat { format, samples })
}

#[test]
fn gpu_a_1_sample_pass_after_a_4_sample_one_on_the_view_is_a_pass_of_its_own() {
    // Given: a red quad at 4 samples, cleared first
    let gpu = gpu();
    let (texture, view) = square(&gpu, SIDE);
    let format = texture.format();
    let four = multisampled(&gpu, format);
    let encoder = FrameEncoder::new(&gpu.device);
    let at_one = target(&gpu, &encoder, &view, SIDE);
    let at_four = Target {
        multisampled: Some(&four),
        ..at_one
    };
    clear_target(&at_four, rgba(BACKGROUND));
    quads_at(&gpu, format, Samples::Four).draw(&at_four, &[quad([0.0, 0.0, 8.0, 16.0], RED)]);

    // When: a blue quad at 1 sample on the same view
    quads_at(&gpu, format, Samples::One).draw(&at_one, &[quad([8.0, 0.0, 16.0, 16.0], BLUE)]);
    encoder.submit(&gpu.queue).expect("no refusal");

    // Then: each in its own pass, both drawn
    let pixels = read_rgba8(&gpu.device, &gpu.queue, &texture);
    assert_eq!(
        (pixel(&pixels, SIDE, 3, 8), pixel(&pixels, SIDE, 12, 8)),
        (RED, BLUE)
    );
}

#[test]
#[should_panic(expected = "quads: a pipeline made for 1 samples drawn into a target of 4")]
fn gpu_a_1_sample_pipeline_drawn_into_a_4_sample_target_is_refused_naming_it() {
    let gpu = gpu();
    let (texture, view) = square(&gpu, SIDE);
    let four = multisampled(&gpu, texture.format());
    let encoder = FrameEncoder::new(&gpu.device);
    let at = Target {
        multisampled: Some(&four),
        ..target(&gpu, &encoder, &view, SIDE)
    };
    clear_target(&at, rgba(BACKGROUND));
    quads_at(&gpu, texture.format(), Samples::One).draw(&at, &[quad([0.0, 0.0, 8.0, 16.0], RED)]);
}

#[test]
#[should_panic(expected = "which this frame's encoder has not cleared")]
fn gpu_a_4_sample_draw_on_a_view_not_cleared_in_the_frame_is_refused() {
    let gpu = gpu();
    let (texture, view) = square(&gpu, SIDE);
    let format = texture.format();
    let four = multisampled(&gpu, format);
    let encoder = FrameEncoder::new(&gpu.device);
    let at = Target {
        multisampled: Some(&four),
        ..target(&gpu, &encoder, &view, SIDE)
    };
    quads_at(&gpu, format, Samples::Four).draw(&at, &[quad([0.0, 0.0, 8.0, 16.0], RED)]);
}
