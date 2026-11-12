//! GPU tier: quads kept between frames (`retained.rs`): written once and
//! drawn by later frames with no write; when no free range holds new
//! bytes, the live ranges move, compacted when half the buffer is free
//! and grown otherwise, and every range still draws its own quads; bytes
//! over the device's `max_buffer_size` are refused.

#[path = "support/frame.rs"]
mod frame;

use frame::{BACKGROUND, pixel, quad, square, target};
use vapors_gpu_harness::{Gpu, gpu, read_rgba8};
use vapors_renderer::{ArenaFull, FrameEncoder, KeptRange, QuadPipeline};

const SIDE: u32 = 4;
/// A quad's instance bytes.
const QUAD_BYTES: u64 = 128;
/// A kept buffer's first size (`FIRST_SIZE` in `retained.rs`): 512 quads.
const FIRST_SIZE: u64 = 64 * 1024;
const RED: [u8; 4] = [255, 0, 0, 255];
const GREEN: [u8; 4] = [0, 255, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];

/// A square to draw on, its pipeline and a frame's encoder.
struct Frame {
    gpu: Gpu,
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    pipeline: QuadPipeline,
}

impl Frame {
    fn new(gpu: Gpu) -> Self {
        let (texture, view) = square(&gpu, SIDE);
        let pipeline = QuadPipeline::new(&gpu.device, texture.format());
        Self {
            gpu,
            texture,
            view,
            pipeline,
        }
    }

    /// `count` quads in `colour` over pixel `(x, 0)`, kept.
    fn keep(
        &mut self,
        encoder: &FrameEncoder,
        (count, x, colour): (usize, f32, [u8; 4]),
    ) -> Option<KeptRange> {
        let quads = vec![quad([x, 0.0, x + 1.0, 1.0], colour); count];
        let at = target(&self.gpu, encoder, &self.view, SIDE);
        self.pipeline.keep(&at, &QuadPipeline::bytes(&quads))
    }

    fn draw(&mut self, encoder: &FrameEncoder, kept: &KeptRange) {
        let at = target(&self.gpu, encoder, &self.view, SIDE);
        self.pipeline.draw_kept(&at, kept);
    }

    /// The colours of the top row's first three pixels.
    fn row(&self) -> [[u8; 4]; 3] {
        let rgba = read_rgba8(&self.gpu.device, &self.gpu.queue, &self.texture);
        [0, 1, 2].map(|x| pixel(&rgba, SIDE, x, 0))
    }

    fn encoder(&self) -> FrameEncoder {
        FrameEncoder::new(&self.gpu.device)
    }
}

/// Kept, or a panic naming `what`.
fn kept(range: Option<KeptRange>, what: &str) -> KeptRange {
    range.unwrap_or_else(|| panic!("{what} refused"))
}

#[test]
fn gpu_kept_quads_draw_in_a_later_frame_with_nothing_written() {
    // Given: a range kept and drawn
    let mut frame = Frame::new(gpu());
    let first = frame.encoder();
    let red = kept(frame.keep(&first, (1, 0.0, RED)), "red");
    frame.draw(&first, &red);
    first.submit(&frame.gpu.queue).expect("no refusal");
    let _ = frame.pipeline.take_work();
    frame.texture = square(&frame.gpu, SIDE).0;
    frame.view = frame
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());

    // When: drawn again in a frame of its own
    let second = frame.encoder();
    frame.draw(&second, &red);
    second.submit(&frame.gpu.queue).expect("no refusal");

    // Then
    let work = frame.pipeline.take_work();
    assert_eq!(frame.row(), [RED, BACKGROUND, BACKGROUND]);
    assert_eq!((work.buffer_bytes, work.draw_calls), (0, 1));
    assert_eq!(frame.pipeline.retained_bytes(), QUAD_BYTES);
}

#[test]
fn gpu_a_full_buffer_half_free_compacts_and_each_range_draws_its_own_quads() {
    // Given: the buffer filled by A, B, C and D, then A and C released,
    // leaving two holes of 160 quads
    let mut frame = Frame::new(gpu());
    let filling = frame.encoder();
    let ranges = [
        (160, 3.0, RED),
        (80, 0.0, GREEN),
        (160, 3.0, RED),
        (112, 1.0, BLUE),
    ];
    let [first, kept_b, third, kept_d] =
        ranges.map(|range| kept(frame.keep(&filling, range), "fill"));
    filling.submit(&frame.gpu.queue).expect("no refusal");
    frame.pipeline.release(first);
    frame.pipeline.release(third);
    let _ = frame.pipeline.take_work();

    // When: B drawn, then 240 quads kept, then D and the new range drawn
    let encoder = frame.encoder();
    frame.draw(&encoder, &kept_b);
    let kept_e = kept(frame.keep(&encoder, (240, 2.0, RED)), "E");
    frame.draw(&encoder, &kept_d);
    frame.draw(&encoder, &kept_e);
    encoder.submit(&frame.gpu.queue).expect("no refusal");

    // Then: B's pixel came from the old buffer, D's and E's from the new
    let work = frame.pipeline.take_work();
    assert_eq!((work.retained_compacted, work.retained_grown), (1, 0));
    assert_eq!(
        frame.pipeline.retained_bytes(),
        (80 + 112 + 240) * QUAD_BYTES
    );
    assert_eq!(frame.row(), [GREEN, BLUE, RED]);
}

#[test]
fn gpu_a_full_buffer_under_half_free_grows_and_each_range_draws_its_own_quads() {
    // Given: 400 quads kept
    let mut frame = Frame::new(gpu());
    let encoder = frame.encoder();
    let first = kept(frame.keep(&encoder, (400, 0.0, RED)), "A");

    // When: 200 more, past the buffer's end
    let second = kept(frame.keep(&encoder, (200, 1.0, GREEN)), "B");
    frame.draw(&encoder, &first);
    frame.draw(&encoder, &second);
    encoder.submit(&frame.gpu.queue).expect("no refusal");

    // Then
    let work = frame.pipeline.take_work();
    assert_eq!((work.retained_grown, work.retained_compacted), (1, 0));
    assert_eq!(frame.row(), [RED, GREEN, BACKGROUND]);
    assert_eq!(frame.pipeline.retained_bytes(), 600 * QUAD_BYTES);
}

#[test]
fn gpu_kept_bytes_over_the_devices_max_buffer_size_are_refused() {
    // Given: a device whose buffers hold no more than the first one
    let mut frame = Frame::new(gpu().with_max_buffer_size(FIRST_SIZE));
    let encoder = frame.encoder();
    let count = FIRST_SIZE / QUAD_BYTES + 1;
    let count_quads = usize::try_from(count).expect("a few hundred");

    // When
    let refused = frame.keep(&encoder, (count_quads, 0.0, RED));
    let submitted = encoder.submit(&frame.gpu.queue);

    // Then
    let full = ArenaFull {
        pipeline: "quads",
        size: count * QUAD_BYTES,
        limit: FIRST_SIZE,
    };
    assert_eq!(refused, None);
    assert_eq!(submitted, Err(full));
    assert_eq!(frame.pipeline.retained_bytes(), 0);
}

#[test]
#[should_panic(expected = "a range kept by another Retained buffer")]
fn gpu_a_range_drawn_by_another_pipeline_panics() {
    // Given: a range kept by one quad pipeline
    let mut frame = Frame::new(gpu());
    let encoder = frame.encoder();
    let red = kept(frame.keep(&encoder, (1, 0.0, RED)), "red");
    frame.pipeline = QuadPipeline::new(&frame.gpu.device, frame.texture.format());

    // When: drawn by another
    frame.draw(&encoder, &red);
}

#[test]
#[should_panic(expected = "is still open")]
fn gpu_bytes_kept_for_a_newer_encoder_while_an_older_is_open_panic() {
    // Given: bytes kept for an encoder not yet submitted
    let mut frame = Frame::new(gpu());
    let older = frame.encoder();
    let _red = frame.keep(&older, (1, 0.0, RED));

    // When
    let newer = frame.encoder();
    let _ = frame.keep(&newer, (1, 1.0, RED));
}
