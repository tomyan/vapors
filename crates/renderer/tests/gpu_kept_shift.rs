//! GPU tier: kept quads and shadows drawn under a scroll frame's shift
//! (IU4 4c, `docs/plans/iu4.md` section 3): bytes kept in the frame's
//! coordinates land moved by the shift, as the same items drawn at the
//! moved place do; a frame drawing them at a new shift writes one
//! uniform value and no instance bytes.

#[path = "support/frame.rs"]
mod frame;
#[path = "support/retained.rs"]
mod retained;

use frame::{BACKGROUND, quad, rgba, square, target};
use retained::{Frame, RED, kept};
use vapors_gpu_harness::{gpu, read_rgba8};
use vapors_renderer::{
    ClosedFormPipeline, ClosedFormShape, FrameEncoder, QuadPipeline, ShadowPipeline, Shift, Target,
};
use vapors_scene::{Color, Rect, Shadow};

const SIDE: u32 = 24;

#[test]
fn gpu_kept_quads_drawn_with_a_shift_match_the_quad_drawn_at_the_moved_place() {
    // Given
    let at = |x: f32, y: f32| [quad([x, y, x + 5.0, y + 3.0], RED)];
    let moved = drawn_on_square(|pipeline: &mut QuadPipeline, target| {
        pipeline.draw(target, &at(7.0, 9.0));
    });

    // When: kept at 4, 4 and drawn shifted by 3, 5
    let shifted = drawn_on_square(|pipeline: &mut QuadPipeline, target| {
        let range = pipeline
            .keep(target, &QuadPipeline::bytes(&at(4.0, 4.0)))
            .expect("room");
        pipeline.draw_kept(target, &range, Shift::whole([3.0, 5.0]));
        pipeline.release(range);
    });

    // Then
    assert!(moved.contains(&255), "the quad drew nothing");
    assert!(
        shifted == moved,
        "the shifted quad differs from the moved one"
    );
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
    frame.draw_shifted(&second, &red, Shift::whole([2.0, 0.0]));
    second.submit(&frame.gpu.queue).expect("no refusal");

    // Then
    let work = frame.pipeline.take_work();
    assert_eq!(frame.row(), [BACKGROUND, BACKGROUND, RED]);
    assert_eq!((work.uniform_writes, work.buffer_bytes), (1, 16));
    frame.pipeline.release(red);
}

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

/// A pipeline made for targets of `format`.
trait Made {
    fn made(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self;
}

impl Made for ShadowPipeline {
    fn made(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        Self::new(device, format)
    }
}

impl Made for ClosedFormPipeline {
    fn made(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        Self::new(device, format)
    }
}

impl Made for QuadPipeline {
    fn made(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        Self::new(device, format)
    }
}

/// The pixels of `draw` on a square `SIDE` across with a pipeline `P`.
fn drawn_on_square<P: Made>(draw: impl FnOnce(&mut P, &Target<'_>)) -> Vec<u8> {
    let gpu = gpu();
    let (texture, view) = square(&gpu, SIDE);
    let mut pipeline = P::made(&gpu.device, texture.format());
    let encoder = FrameEncoder::new(&gpu.device);
    draw(&mut pipeline, &target(&gpu, &encoder, &view, SIDE));
    encoder.submit(&gpu.queue).expect("no refusal");
    read_rgba8(&gpu.device, &gpu.queue, &texture)
}

#[test]
fn gpu_kept_shadows_drawn_with_a_shift_match_the_shadow_drawn_at_the_moved_place() {
    // Given
    let moved =
        drawn_on_square(|pipeline: &mut ShadowPipeline, at| pipeline.draw(at, &[shadow(7.0, 9.0)]));

    // When: kept at 4, 4 and drawn shifted by 3, 5
    let shifted = drawn_on_square(|pipeline: &mut ShadowPipeline, at| {
        let bytes = ShadowPipeline::bytes(&[shadow(4.0, 4.0)]);
        let range = pipeline.keep(at, &bytes).expect("room");
        pipeline.draw_kept(at, &range, Shift::whole([3.0, 5.0]));
        pipeline.release(range);
    });

    // Then
    assert!(moved.iter().any(|&v| v != 40), "the shadow drew nothing");
    assert!(
        shifted == moved,
        "the shifted shadow differs from the moved one"
    );
}

/// A circle at `x`, `y` drawn under a transform of scale 1.5.
fn circle(x: f32, y: f32) -> ClosedFormShape {
    let geometry = vapors_shapes::Geometry::Ellipse {
        centre: [x / 1.5, y / 1.5],
        radii: [2.0, 2.0],
    };
    let red = Color::from_rgba8(200, 30, 20, 255);
    ClosedFormShape::new(&geometry, None, [1.5, 0.0, 0.0, 1.5, 0.0, 0.0], red)
        .expect("drawable")
        .expect("has area")
}

#[test]
fn gpu_kept_closed_forms_drawn_with_a_shift_match_the_shape_drawn_at_the_moved_place() {
    // Given
    let moved = drawn_on_square(|pipeline: &mut ClosedFormPipeline, at| {
        pipeline.draw(at, &[circle(7.0, 9.0)]);
    });

    // When: kept at 4, 4 and drawn shifted by 3, 5
    let shifted = drawn_on_square(|pipeline: &mut ClosedFormPipeline, at| {
        let bytes = ClosedFormPipeline::bytes(&[circle(4.0, 4.0)]);
        let range = pipeline.keep(at, &bytes).expect("room");
        pipeline.draw_kept(at, &range, Shift::whole([3.0, 5.0]));
        pipeline.release(range);
    });

    // Then
    assert!(moved.iter().any(|&v| v != 40), "the circle drew nothing");
    assert!(
        shifted == moved,
        "the shifted circle differs from the moved one"
    );
}
