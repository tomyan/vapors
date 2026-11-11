//! GPU tier: consecutive draws on one view share one render pass, begun
//! by a clear or the first draw; a draw on another view, a raw record
//! and a flush end it; a draw with no scissor covers the whole view
//! after one with a scissor.

#[path = "support/frame.rs"]
mod frame;

use frame::{BACKGROUND, pixel, quad, rgba, square, target};
use vapors_gpu_harness::{gpu, read_rgba8};
use vapors_renderer::{FrameEncoder, QuadPipeline, clear_target};
use vapors_scene::Scissor;

const RED: [u8; 4] = [255, 0, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];

#[test]
fn gpu_a_clear_and_many_draws_on_one_view_record_one_render_pass() {
    // Given
    let gpu = gpu();
    let (texture, view) = square(&gpu, 8);
    let mut pipeline = QuadPipeline::new(&gpu.device, texture.format());
    let encoder = FrameEncoder::new(&gpu.device);
    let on_view = target(&gpu, &encoder, &view, 8);

    // When: a clear, then a draw per pixel of the top row
    clear_target(&on_view, rgba(BLUE));
    for x in 0..8u8 {
        let left = f32::from(x);
        pipeline.draw(&on_view, &[quad([left, 0.0, left + 1.0, 1.0], RED)]);
    }
    let passes = encoder.render_passes();
    encoder.submit(&gpu.queue).expect("no draw refused");

    // Then
    let pixels = read_rgba8(&gpu.device, &gpu.queue, &texture);
    assert_eq!(passes, 1);
    let row: Vec<[u8; 4]> = (0..8).map(|x| pixel(&pixels, 8, x, 0)).collect();
    assert_eq!((row, pixel(&pixels, 8, 0, 1)), (vec![RED; 8], BLUE));
}

#[test]
fn gpu_each_change_of_view_ends_the_pass_and_begins_one_on_the_new_view() {
    // Given
    let gpu = gpu();
    let ((first, first_view), (second, second_view)) = (square(&gpu, 4), square(&gpu, 4));
    let mut pipeline = QuadPipeline::new(&gpu.device, first.format());
    let encoder = FrameEncoder::new(&gpu.device);

    // When: first, first, second, first
    for (view, colour) in [
        (&first_view, RED),
        (&first_view, RED),
        (&second_view, BLUE),
        (&first_view, RED),
    ] {
        let on = target(&gpu, &encoder, view, 4);
        pipeline.draw(&on, &[quad([0.0, 0.0, 4.0, 4.0], colour)]);
    }
    let passes = encoder.render_passes();
    encoder.submit(&gpu.queue).expect("no draw refused");

    // Then
    let first = read_rgba8(&gpu.device, &gpu.queue, &first);
    let second = read_rgba8(&gpu.device, &gpu.queue, &second);
    assert_eq!(passes, 3);
    assert_eq!(
        (pixel(&first, 4, 1, 1), pixel(&second, 4, 1, 1)),
        (RED, BLUE)
    );
}

#[test]
fn gpu_a_raw_record_and_a_flush_each_end_the_open_pass() {
    // Given
    let gpu = gpu();
    let (texture, view) = square(&gpu, 4);
    let mut pipeline = QuadPipeline::new(&gpu.device, texture.format());
    let encoder = FrameEncoder::new(&gpu.device);
    let on_view = target(&gpu, &encoder, &view, 4);
    let draw = |pipeline: &mut QuadPipeline| {
        pipeline.draw(&on_view, &[quad([0.0, 0.0, 4.0, 4.0], RED)]);
    };

    // When
    draw(&mut pipeline);
    encoder.record(|_| ());
    draw(&mut pipeline);
    encoder.flush(&gpu.queue);
    draw(&mut pipeline);
    let passes = encoder.render_passes();
    encoder.submit(&gpu.queue).expect("no draw refused");

    // Then
    assert_eq!(passes, 3);
}

#[test]
fn gpu_a_draw_with_no_scissor_after_a_scissored_one_covers_the_whole_view() {
    // Given
    let gpu = gpu();
    let (texture, view) = square(&gpu, 4);
    let mut pipeline = QuadPipeline::new(&gpu.device, texture.format());
    let encoder = FrameEncoder::new(&gpu.device);
    let whole = target(&gpu, &encoder, &view, 4);
    let corner = Scissor {
        x: 0,
        y: 0,
        width: 1,
        height: 1,
    };
    let scissored = vapors_renderer::Target {
        scissor: Some(corner),
        ..whole
    };

    // When
    pipeline.draw(&scissored, &[quad([0.0, 0.0, 4.0, 4.0], RED)]);
    pipeline.draw(&whole, &[quad([2.0, 2.0, 4.0, 4.0], BLUE)]);
    encoder.submit(&gpu.queue).expect("no draw refused");

    // Then
    let pixels = read_rgba8(&gpu.device, &gpu.queue, &texture);
    let seen = [(0, 0), (1, 1), (3, 3)].map(|(x, y)| pixel(&pixels, 4, x, y));
    assert_eq!(seen, [RED, BACKGROUND, BLUE]);
}
