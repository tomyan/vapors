//! GPU tier: quads kept between frames (`retained.rs`): written once and
//! drawn by later frames with no write; a range released in the frame
//! that drew it is not reused there, whether the frame kept bytes before
//! the draw or not; bytes over the device's `max_buffer_size` are
//! refused, naming the kept bytes; a range drawn by another pipeline,
//! bytes kept for a newer encoder while an older is open, and bytes not
//! a whole number of quads panic.

#[path = "support/frame.rs"]
mod frame;
#[path = "support/retained.rs"]
mod retained;

use frame::{BACKGROUND, square, target};
use retained::{FIRST_SIZE, Frame, GREEN, QUAD_BYTES, RED, SIDE, kept};
use vapors_gpu_harness::gpu;
use vapors_renderer::{ArenaFull, Holding, QuadPipeline};

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
        holding: Holding::Kept,
    };
    assert_eq!(refused, None);
    assert!(full.to_string().contains("release kept ranges"), "{full}");
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

#[test]
fn gpu_a_range_released_in_the_frame_that_drew_it_is_not_reused_in_that_frame() {
    // Given: a red quad kept, drawn and released in one frame
    let mut frame = Frame::new(gpu());
    let encoder = frame.encoder();
    let red = kept(frame.keep(&encoder, (1, 0.0, RED)), "red");
    frame.draw(&encoder, &red);
    frame.pipeline.release(red);

    // When: a green one kept and drawn in the same frame
    let green = kept(frame.keep(&encoder, (1, 1.0, GREEN)), "green");
    frame.draw(&encoder, &green);
    encoder.submit(&frame.gpu.queue).expect("no refusal");

    // Then: the red draw read its own bytes
    assert_eq!(frame.row(), [RED, GREEN, BACKGROUND]);
}

#[test]
fn gpu_a_range_drawn_then_released_before_its_frame_keeps_anything_is_not_reused_in_that_frame() {
    // Given: a red quad kept in one frame
    let mut frame = Frame::new(gpu());
    let first = frame.encoder();
    let red = kept(frame.keep(&first, (1, 0.0, RED)), "red");
    first.submit(&frame.gpu.queue).expect("no refusal");

    // When: the next frame draws and releases it, then keeps a green
    // one and draws that
    let second = frame.encoder();
    frame.draw(&second, &red);
    frame.pipeline.release(red);
    let green = kept(frame.keep(&second, (1, 1.0, GREEN)), "green");
    frame.draw(&second, &green);
    second.submit(&frame.gpu.queue).expect("no refusal");

    // Then: the red draw read its own bytes
    assert_eq!(frame.row(), [RED, GREEN, BACKGROUND]);
}

#[test]
#[should_panic(expected = "not a whole number of 128-byte instances")]
fn gpu_kept_bytes_not_a_whole_number_of_quads_panic() {
    // Given
    let mut frame = Frame::new(gpu());
    let encoder = frame.encoder();
    let at = target(&frame.gpu, &encoder, &frame.view, SIDE);

    // When: three 16-byte rows, part of one quad
    let _ = frame.pipeline.keep(&at, &[0; 48]);
}
