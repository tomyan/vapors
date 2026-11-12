//! GPU tier: an image pipeline's quads kept (`image_kept.rs`): a run's
//! rects written once, each drawn by its place in the kept range with
//! its own texture, in a later frame with nothing written; a place past
//! the range panics.

use vapors_gpu_harness::{Gpu, gpu, read_rgba8};
use vapors_renderer::{
    FrameEncoder, ImagePipeline, KeptRange, Sampling, Target, TextureBinding, clear, upload_rgba8,
};
use vapors_scene::{Color, Rect, Size};

const RED: [u8; 4] = [211, 47, 47, 255];
const BLUE: [u8; 4] = [25, 118, 210, 255];
const VIEWPORT: Size = Size {
    width: 16,
    height: 8,
};

fn pixel(pixels: &[u8], x: u32, y: u32) -> [u8; 4] {
    let start = ((y * VIEWPORT.width + x) * 4) as usize;
    pixels[start..start + 4].try_into().expect("four bytes")
}

/// A grey target, the pipeline, and a red and a blue image bound.
struct Scene {
    gpu: Gpu,
    target: wgpu::Texture,
    pipeline: ImagePipeline,
    red: TextureBinding,
    blue: TextureBinding,
}

impl Scene {
    fn new() -> Self {
        let gpu = gpu();
        let grey = Color::from_rgba8(40, 40, 40, 255);
        let target = clear(&gpu.device, &gpu.queue, 16, 8, grey);
        let pipeline = ImagePipeline::new(&gpu.device, target.format());
        let bound = |rgba: [u8; 4]| {
            let image = upload_rgba8(&gpu.device, &gpu.queue, 1, 1, &rgba);
            pipeline.bind(&gpu.device, &image, Sampling::Smooth)
        };
        let (red, blue) = (bound(RED), bound(BLUE));
        Self {
            gpu,
            target,
            pipeline,
            red,
            blue,
        }
    }

    /// Runs `frame` with a target on the grey texture, then submits.
    fn frame(&mut self, frame: impl FnOnce(&mut Self, &Target<'_>)) {
        let view = self
            .target
            .create_view(&wgpu::TextureViewDescriptor::default());
        let encoder = FrameEncoder::new(&self.gpu.device);
        let device = self.gpu.device.clone();
        let queue = self.gpu.queue.clone();
        let at = Target {
            device: &device,
            queue: &queue,
            encoder: &encoder,
            view: &view,
            viewport: VIEWPORT,
            scissor: None,
        };
        frame(self, &at);
        encoder.submit(&queue).expect("no draw refused");
    }

    /// The left and right halves' rects, kept.
    fn keep_halves(&mut self, at: &Target<'_>) -> KeptRange {
        let halves = [
            Rect::new(0.0, 0.0, 8.0, 8.0),
            Rect::new(8.0, 0.0, 16.0, 8.0),
        ];
        let bytes = ImagePipeline::rect_bytes(VIEWPORT, halves);
        self.pipeline.keep(at, &bytes).expect("room")
    }
}

#[test]
fn gpu_kept_image_quads_draw_by_their_place_in_a_later_frame_with_nothing_written() {
    // Given: the halves kept in one frame
    let mut scene = Scene::new();
    let mut kept = None;
    scene.frame(|scene, at| kept = Some(scene.keep_halves(at)));
    let (kept, written) = (kept.expect("kept"), scene.pipeline.take_work());

    // When: blue drawn over the right half, red over the left
    scene.frame(|scene, at| {
        scene.pipeline.draw_kept(at, &scene.blue, (&kept, 1));
        scene.pipeline.draw_kept(at, &scene.red, (&kept, 0));
    });

    // Then
    let work = scene.pipeline.take_work();
    let pixels = read_rgba8(&scene.gpu.device, &scene.gpu.queue, &scene.target);
    assert_eq!([pixel(&pixels, 2, 2), pixel(&pixels, 12, 5)], [RED, BLUE]);
    assert_eq!(written.buffer_bytes, 2 * 6 * 16);
    assert_eq!((work.buffer_bytes, work.draw_calls), (0, 2));
    assert_eq!(scene.pipeline.retained_bytes(), 2 * 6 * 16);
}

#[test]
#[should_panic(expected = "quad 2 drawn from a kept range of 2 quads")]
fn gpu_a_kept_image_quad_past_the_range_panics() {
    // Given
    let mut scene = Scene::new();

    // When
    scene.frame(|scene, at| {
        let kept = scene.keep_halves(at);
        scene.pipeline.draw_kept(at, &scene.red, (&kept, 2));
    });
}
