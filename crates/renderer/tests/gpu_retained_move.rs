//! GPU tier: kept quads moved (`retained_move.rs`): when no free range
//! holds new bytes, the live ranges are compacted when half the buffer
//! is free after the move (ranges released this frame counted) and
//! grown otherwise, and every range still draws its own quads; the
//! first buffer holds `FIRST_SIZE` bytes; released ranges are reused
//! once a newer encoder writes, and are left behind by a move.

#[path = "support/frame.rs"]
mod frame;
#[path = "support/retained.rs"]
mod retained;

use frame::{BACKGROUND, quad};
use retained::{BLUE, FIRST_SIZE, Frame, GREEN, QUAD_BYTES, RED, kept};
use vapors_gpu_harness::gpu;
use vapors_renderer::QuadPipeline;

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
fn gpu_a_released_range_is_reused_once_a_newer_encoder_writes() {
    // Given: a quad kept and released in one frame
    let mut frame = Frame::new(gpu());
    let first = frame.encoder();
    let red = kept(frame.keep(&first, (512, 0.0, RED)), "red");
    frame.pipeline.release(red);
    first.submit(&frame.gpu.queue).expect("no refusal");

    // When: a whole buffer's worth kept in the next
    let second = frame.encoder();
    let green = kept(frame.keep(&second, (512, 1.0, GREEN)), "green");
    frame.draw(&second, &green);
    second.submit(&frame.gpu.queue).expect("no refusal");

    // Then: it fit where the first was, with no move
    let work = frame.pipeline.take_work();
    assert_eq!((work.retained_grown, work.retained_compacted), (0, 0));
    assert_eq!(frame.row(), [BACKGROUND, GREEN, BACKGROUND]);
}

#[test]
fn gpu_a_range_released_before_a_move_is_not_given_back_in_the_new_buffer() {
    // Given: A released, then a move that packs B at the start
    let mut frame = Frame::new(gpu());
    let first = frame.encoder();
    let released = kept(frame.keep(&first, (300, 3.0, RED)), "A");
    let green = kept(frame.keep(&first, (100, 0.0, GREEN)), "B");
    frame.pipeline.release(released);
    let _blue = kept(frame.keep(&first, (300, 2.0, BLUE)), "C");
    first.submit(&frame.gpu.queue).expect("no refusal");

    // When: more kept in the next frame, and B drawn beside it
    let second = frame.encoder();
    let red = kept(frame.keep(&second, (100, 1.0, RED)), "D");
    frame.draw(&second, &green);
    frame.draw(&second, &red);
    second.submit(&frame.gpu.queue).expect("no refusal");

    // Then: B kept its own bytes
    assert_eq!(frame.row(), [GREEN, RED, BACKGROUND]);
}

#[test]
fn gpu_a_full_buffer_mostly_released_in_the_frame_compacts_rather_than_grows() {
    // Given: 400 quads kept in one frame
    let mut frame = Frame::new(gpu());
    let first = frame.encoder();
    let many = kept(frame.keep(&first, (400, 2.0, RED)), "A");
    first.submit(&frame.gpu.queue).expect("no refusal");
    let _ = frame.pipeline.take_work();

    // When: the next keeps one quad, releases the 400, then keeps 200
    let second = frame.encoder();
    let one = kept(frame.keep(&second, (1, 0.0, GREEN)), "B");
    frame.pipeline.release(many);
    let more = kept(frame.keep(&second, (200, 1.0, BLUE)), "C");
    frame.draw(&second, &one);
    frame.draw(&second, &more);
    second.submit(&frame.gpu.queue).expect("no refusal");

    // Then: 201 quads fit a buffer of 512, so it compacted
    let work = frame.pipeline.take_work();
    assert_eq!((work.retained_compacted, work.retained_grown), (1, 0));
    assert_eq!(frame.row(), [GREEN, BLUE, BACKGROUND]);
}

#[test]
fn gpu_the_first_kept_buffer_holds_first_size_bytes_of_quads() {
    // Given: the support's sizes are the pipeline's
    let one_quad = QuadPipeline::bytes(&[quad([0.0, 0.0, 1.0, 1.0], RED)]);
    assert_eq!(one_quad.len() as u64, QUAD_BYTES);
    let mut frame = Frame::new(gpu());
    let encoder = frame.encoder();
    let whole = usize::try_from(FIRST_SIZE / QUAD_BYTES).expect("a few hundred");

    // When: a buffer's worth kept, then one more quad
    let _all = kept(frame.keep(&encoder, (whole, 0.0, RED)), "all");
    let filled = frame.pipeline.take_work();
    let _one = kept(frame.keep(&encoder, (1, 0.0, RED)), "one");
    let past = frame.pipeline.take_work();

    // Then: the buffer's worth fit; the quad past it grew the buffer
    assert_eq!((filled.retained_grown, past.retained_grown), (0, 1));
}
