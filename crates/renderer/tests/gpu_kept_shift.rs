//! GPU tier: kept quads and shadows drawn under a scroll frame's shift
//! (IU4 4c, `docs/plans/iu4.md` section 3): bytes kept in the frame's
//! coordinates land moved by the shift, as the same items drawn at the
//! moved place do; a frame drawing them at a new shift writes one
//! uniform value and no instance bytes.

#[path = "support/frame.rs"]
mod frame;
#[path = "support/retained.rs"]
mod retained;

use frame::{BACKGROUND, rgba, square, target};
use retained::{Frame, RED, kept};
use vapors_gpu_harness::{gpu, read_rgba8};
use vapors_renderer::{FrameEncoder, ShadowPipeline};
use vapors_scene::{Rect, Shadow};

#[test]
fn gpu_kept_quads_drawn_with_a_shift_land_moved_by_it() {
    // Given: a red quad over pixel 0, kept
    let mut frame = Frame::new(gpu());
    let encoder = frame.encoder();
    let red = kept(frame.keep(&encoder, (1, 0.0, RED)), "red");

    // When
    frame.draw_shifted(&encoder, &red, [1.0, 0.0]);
    encoder.submit(&frame.gpu.queue).expect("no refusal");

    // Then
    assert_eq!(frame.row(), [BACKGROUND, RED, BACKGROUND]);
    frame.pipeline.release(red);
}

#[test]
fn gpu_kept_quads_drawn_at_a_new_shift_write_one_uniform_and_no_instances() {
    // Given: a red quad kept and drawn unshifted in one frame
    let mut frame = Frame::new(gpu());
    let first = frame.encoder();
    let red = kept(frame.keep(&first, (1, 0.0, RED)), "red");
    frame.draw(&first, &red);
    first.submit(&frame.gpu.queue).expect("no refusal");
    let _ = frame.pipeline.take_work();
    frame.texture = square(&frame.gpu, retained::SIDE).0;
    frame.view = frame
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());

    // When: drawn two pixels right in the next
    let second = frame.encoder();
    frame.draw_shifted(&second, &red, [2.0, 0.0]);
    second.submit(&frame.gpu.queue).expect("no refusal");

    // Then
    let work = frame.pipeline.take_work();
    assert_eq!(frame.row(), [BACKGROUND, BACKGROUND, RED]);
    assert_eq!((work.uniform_writes, work.buffer_bytes), (1, 16));
    frame.pipeline.release(red);
}

const SIDE: u32 = 24;

/// A blurred shadow of a box at `x`, `y`, cast by the box.
fn shadow(x: f32, y: f32) -> Shadow {
    let rect = Rect::new(x, y, x + 8.0, y + 6.0);
    Shadow {
        rect: rect.offset([1.0, 2.0]),
        radii: [2.0; 4],
        blur: 4.0,
        color: rgba([0, 0, 0, 200]),
        caster: rect,
        caster_radii: [2.0; 4],
    }
}

/// The pixels of `draw` on a square `SIDE` across with a shadow pipeline.
fn shadows_drawn(draw: impl FnOnce(&mut ShadowPipeline, &vapors_renderer::Target<'_>)) -> Vec<u8> {
    let gpu = gpu();
    let (texture, view) = square(&gpu, SIDE);
    let mut pipeline = ShadowPipeline::new(&gpu.device, texture.format());
    let encoder = FrameEncoder::new(&gpu.device);
    draw(&mut pipeline, &target(&gpu, &encoder, &view, SIDE));
    encoder.submit(&gpu.queue).expect("no refusal");
    read_rgba8(&gpu.device, &gpu.queue, &texture)
}

#[test]
fn gpu_kept_shadows_drawn_with_a_shift_match_the_shadow_drawn_at_the_moved_place() {
    // Given
    let moved = shadows_drawn(|pipeline, at| pipeline.draw(at, &[shadow(7.0, 9.0)]));

    // When: kept at 4, 4 and drawn shifted by 3, 5
    let shifted = shadows_drawn(|pipeline, at| {
        let bytes = ShadowPipeline::bytes(&[shadow(4.0, 4.0)]);
        let range = pipeline.keep(at, &bytes).expect("room");
        pipeline.draw_kept(at, &range, [3.0, 5.0]);
        pipeline.release(range);
    });

    // Then
    assert!(moved.iter().any(|&v| v != 40), "the shadow drew nothing");
    assert!(
        shifted == moved,
        "the shifted shadow differs from the moved one"
    );
}
