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
use vapors_renderer::{FrameEncoder, QuadPipeline, ShadowPipeline, TransformTable, translation};
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

/// The red quad 3 by 2 at `x`, `y`.
fn red(x: f32, y: f32) -> vapors_scene::Quad {
    quad([x, y, x + 3.0, y + 2.0], [255, 0, 0, 255])
}

#[test]
fn gpu_a_kept_quad_drawn_after_the_table_grows_lands_at_its_rows_offset() {
    // Given: the quad drawn where its frame and row put it
    let moved = drawn(|device, format, target| {
        QuadPipeline::new(device, format).draw(target, &[red(5.0, 13.0)]);
    });
    let gpu = gpu();
    // and a first frame drawing from the table's first buffer, so the
    // pipeline keeps a bind group for it
    let (first_texture, first_view) = square(&gpu, SIDE);
    let mut pipeline = QuadPipeline::new(&gpu.device, first_texture.format());
    let mut table = TransformTable::new(&gpu.device, "growing transforms");
    let first = FrameEncoder::new(&gpu.device);
    let at = target(&gpu, &first, &first_view, SIDE);
    let frame = table.take(&first, 1).expect("room");
    table.set(&first, &gpu.queue, frame, &[translation([3.0, 2.0])]);
    let early = pipeline
        .keep(&at, &QuadPipeline::bytes_in(&[red(0.0, 0.0)], frame, &[]))
        .expect("room");
    pipeline.draw_kept(&at, &early, (&mut table, 0));
    first.submit(&gpu.queue).expect("no refusal");

    // When: the next frame takes 70 more entries, past the first buffer's
    // 64, and draws a quad kept in the last of them, 10 px down
    let (texture, view) = square(&gpu, SIDE);
    let second = FrameEncoder::new(&gpu.device);
    let at = target(&gpu, &second, &view, SIDE);
    let base = table.take(&second, 70).expect("the table grows");
    let mut rows = vec![translation([0.0, 0.0]); 70];
    rows[69] = translation([0.0, 10.0]);
    table.set(&second, &gpu.queue, base, &rows);
    let late = pipeline
        .keep(&at, &QuadPipeline::bytes_in(&[red(2.0, 1.0)], frame, &[69]))
        .expect("room");
    pipeline.draw_kept(&at, &late, (&mut table, base));
    second.submit(&gpu.queue).expect("no refusal");

    // Then
    assert!(base + 69 >= 64, "the row's entry is past the first buffer");
    let grown = read_rgba8(&gpu.device, &gpu.queue, &texture);
    assert!(moved.contains(&255), "the quad drew nothing");
    assert!(
        grown == moved,
        "the quad drawn after the table grew differs"
    );
    pipeline.release(early);
    pipeline.release(late);
}

#[test]
#[should_panic(expected = "a pipeline draws from one table")]
fn gpu_a_pipeline_given_a_second_transform_table_refuses_it() {
    let gpu = gpu();
    let (texture, view) = square(&gpu, SIDE);
    let mut pipeline = QuadPipeline::new(&gpu.device, texture.format());
    let encoder = FrameEncoder::new(&gpu.device);
    let at = target(&gpu, &encoder, &view, SIDE);
    let range = pipeline
        .keep(&at, &QuadPipeline::bytes(&[red(0.0, 0.0)]))
        .expect("room");
    let mut first = TransformTable::new(&gpu.device, "first");
    let mut second = TransformTable::new(&gpu.device, "second");
    pipeline.draw_kept(&at, &range, (&mut first, 0));
    pipeline.draw_kept(&at, &range, (&mut second, 0));
}
