//! GPU tier: kept quads and shadows in the rows of a lane
//! (`docs/plans/splice-layout.md` section 11.2, 4f), placed through the
//! transform table (`docs/plans/flat-scene-f2.md`): each instance moved by
//! its frame's entry and its own row's entry lands where the same item
//! drawn at the moved place does, so items of several rows draw in one
//! draw.

#[path = "support/frame.rs"]
mod frame;
#[path = "support/table.rs"]
mod table;

use frame::{quad, rgba, square, target};
use table::table;
use vapors_gpu_harness::{gpu, read_rgba8};
use vapors_renderer::{FrameEncoder, QuadPipeline, ShadowPipeline};
use vapors_scene::{Rect, Shadow};

const SIDE: u32 = 24;

/// What `draw` draws on a fresh square, as RGBA bytes.
fn drawn(
    draw: impl FnOnce(&wgpu::Device, wgpu::TextureFormat, &vapors_renderer::Target<'_>),
) -> Vec<u8> {
    let gpu = gpu();
    let (texture, view) = square(&gpu, SIDE);
    let encoder = FrameEncoder::new(&gpu.device);
    draw(
        &gpu.device,
        texture.format(),
        &target(&gpu, &encoder, &view, SIDE),
    );
    encoder.submit(&gpu.queue).expect("no refusal");
    read_rgba8(&gpu.device, &gpu.queue, &texture)
}

#[test]
fn gpu_kept_quads_in_two_rows_land_each_at_its_rows_offset() {
    // Given: two quads drawn where their rows put them
    let at = |x: f32, y: f32| quad([x, y, x + 3.0, y + 2.0], [255, 0, 0, 255]);
    let moved = drawn(|device, format, target| {
        QuadPipeline::new(device, format).draw(target, &[at(5.0, 3.0), at(5.0, 13.0)]);
    });

    // When: both kept at 2, 1 in their rows' frames, row 1 ten pixels
    // below row 0, the frame shifted by 3, 2
    let rowed = drawn(|device, format, target| {
        let mut pipeline = QuadPipeline::new(device, format);
        let mut table = table(target, [3.0, 2.0], &[[0.0, 0.0], [0.0, 10.0]]);
        let items = [at(2.0, 1.0), at(2.0, 1.0)];
        let bytes = QuadPipeline::bytes_in(&items, table.frame, &[0, 1]);
        let range = pipeline.keep(target, &bytes).expect("room");
        pipeline.draw_kept(target, &range, (&mut table.transforms, table.rows_base));
        pipeline.release(range);
    });

    // Then
    assert!(moved.contains(&255), "the quads drew nothing");
    assert!(
        rowed == moved,
        "the quads in rows differ from the moved ones"
    );
}

/// A blurred shadow of a 6 by 4 caster at `x`, `y`.
fn shadow(x: f32, y: f32) -> Shadow {
    let rect = Rect::new(x, y, x + 6.0, y + 4.0);
    Shadow {
        rect: rect.offset([1.0, 1.0]),
        radii: [1.0; 4],
        blur: 2.0,
        color: rgba([0, 0, 0, 200]),
        caster: rect,
        caster_radii: [1.0; 4],
    }
}

#[test]
fn gpu_kept_shadows_in_two_rows_land_each_at_its_rows_offset() {
    // Given
    let moved = drawn(|device, format, target| {
        ShadowPipeline::new(device, format).draw(target, &[shadow(4.0, 2.0), shadow(4.0, 14.0)]);
    });

    // When: kept at the origin, rows 2 and 14 below it
    let rowed = drawn(|device, format, target| {
        let mut pipeline = ShadowPipeline::new(device, format);
        let mut table = table(target, [4.0, 0.0], &[[0.0, 2.0], [0.0, 14.0]]);
        let items = [shadow(0.0, 0.0), shadow(0.0, 0.0)];
        let bytes = ShadowPipeline::bytes_in(&items, table.frame, &[0, 1]);
        let range = pipeline.keep(target, &bytes).expect("room");
        pipeline.draw_kept(target, &range, (&mut table.transforms, table.rows_base));
        pipeline.release(range);
    });

    // Then
    assert!(moved.iter().any(|&v| v != 40), "the shadows drew nothing");
    assert!(
        rowed == moved,
        "the shadows in rows differ from the moved ones"
    );
}
