//! GPU tier: draws recorded into one `FrameEncoder` and submitted
//! together each read their own data, past the pipeline's first
//! buffers too; a draw for an encoder older than one already drawn
//! with is refused.

use vapors_gpu_harness::{gpu, read_rgba8};
use vapors_renderer::{FrameEncoder, QuadPipeline, Target, clear};
use vapors_scene::{Color, Quad, Rect, Size};

/// More draws than the first 64 KiB instance buffer holds (a quad is
/// 128 bytes: 512 draws), so the arena grows mid-frame.
const DRAWS: u32 = 600;
const SIDE: u32 = 32;
const BACKGROUND: [u8; 4] = [40, 40, 40, 255];

/// Draw `index`'s colour: its own red, so a draw that read another
/// draw's data shows.
fn colour(index: u32) -> [u8; 4] {
    let red = u8::try_from(index % 250).expect("under 250") + 5;
    [red, 0, 0, 255]
}

/// The one-pixel quad of draw `index`, at `(index % SIDE, index / SIDE)`.
#[allow(clippy::cast_precision_loss)] // small pixel positions
fn quad(index: u32) -> Quad {
    let (x, y) = ((index % SIDE) as f32, (index / SIDE) as f32);
    Quad::fill(Rect::new(x, y, x + 1.0, y + 1.0), rgba(colour(index)))
}

fn rgba([red, green, blue, alpha]: [u8; 4]) -> Color {
    Color::from_rgba8(red, green, blue, alpha)
}

fn target<'a>(
    gpu: &'a vapors_gpu_harness::Gpu,
    encoder: &'a FrameEncoder,
    view: &'a wgpu::TextureView,
) -> Target<'a> {
    Target {
        device: &gpu.device,
        queue: &gpu.queue,
        encoder,
        view,
        viewport: Size {
            width: SIDE,
            height: SIDE,
        },
        scissor: None,
    }
}

#[test]
fn gpu_every_draw_of_one_encoder_reads_its_own_instances_and_viewport() {
    // Given
    let gpu = gpu();
    let texture = clear(&gpu.device, &gpu.queue, SIDE, SIDE, rgba(BACKGROUND));
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let mut pipeline = QuadPipeline::new(&gpu.device, texture.format());
    let encoder = FrameEncoder::new(&gpu.device);

    // When: one draw call per quad, one submit
    for index in 0..DRAWS {
        pipeline.draw(&target(&gpu, &encoder, &view), &[quad(index)]);
    }
    encoder.submit(&gpu.queue);

    // Then
    let pixels = read_rgba8(&gpu.device, &gpu.queue, &texture);
    let wrong: Vec<u32> = (0..DRAWS)
        .filter(|&i| pixels[(i * 4) as usize..(i * 4 + 4) as usize] != colour(i))
        .collect();
    assert!(wrong.is_empty(), "draws that read other data: {wrong:?}");
}

#[test]
#[should_panic(expected = "a draw for an encoder older than one already drawn with")]
fn gpu_a_draw_for_an_older_encoder_after_a_newer_one_panics() {
    // Given
    let gpu = gpu();
    let texture = clear(&gpu.device, &gpu.queue, SIDE, SIDE, Color::default());
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let mut pipeline = QuadPipeline::new(&gpu.device, texture.format());
    let (older, newer) = (
        FrameEncoder::new(&gpu.device),
        FrameEncoder::new(&gpu.device),
    );
    pipeline.draw(&target(&gpu, &newer, &view), &[quad(0)]);

    // When
    pipeline.draw(&target(&gpu, &older, &view), &[quad(1)]);
}
