//! GPU tier: text vertices kept between frames (IU4 PR 3) draw the
//! pixels the immediate draw does, and a frame drawing them again
//! writes no vertex bytes and looks nothing up in the atlas.

#[path = "support/painter.rs"]
mod painter;

use std::cell::RefCell;

use painter::Painter;
use vapors_renderer::KeptRange;
use vapors_text::{AtlasPins, BlendParams, TextDraw, TextPipeline};

const PAGE: u32 = 128;

/// The first frame: `text`'s bytes built, pinned, kept and drawn.
fn keep_and_draw(painter: &mut Painter, text: &str) -> (Vec<u8>, KeptRange, AtlasPins) {
    let layout = painter.layout(text);
    let kept = RefCell::new(None);
    let drawn = painter.frame(|pipeline, atlas, target| {
        let draw = TextDraw {
            layout: &layout,
            origin: [2.0, 4.0],
        };
        let (bytes, pins) = TextPipeline::text_bytes(atlas, &[draw])?;
        let range = pipeline.keep(target, &bytes).expect("kept");
        pipeline.draw_kept(target, atlas, &range, BlendParams::default());
        *kept.borrow_mut() = Some((range, pins));
        Ok(())
    });
    let (range, pins) = kept.into_inner().expect("kept in the frame");
    (drawn.pixels, range, pins)
}

/// A later frame drawing `range` again: its pixels and the vertex
/// bytes it wrote.
fn draw_again(painter: &mut Painter, range: &KeptRange) -> (Vec<u8>, u64) {
    let written = RefCell::new(0);
    let drawn = painter.frame(|pipeline, atlas, target| {
        pipeline.draw_kept(target, atlas, range, BlendParams::default());
        *written.borrow_mut() = pipeline.take_work().buffer_bytes;
        Ok(())
    });
    (drawn.pixels, written.into_inner())
}

#[test]
fn gpu_kept_text_draws_as_the_immediate_draw_and_again_with_no_writes_or_lookups() {
    // Given
    let immediate = Painter::new(PAGE).text("Hello").pixels;
    let mut painter = Painter::new(PAGE);
    let (first, range, pins) = keep_and_draw(&mut painter, "Hello");
    let lookups = painter.atlas.lookups();

    // When
    let (again, written) = draw_again(&mut painter, &range);

    // Then
    assert!(
        first == immediate,
        "kept text differs from the immediate draw"
    );
    assert!(again == immediate, "kept text drawn again differs");
    assert_eq!(written, 0, "a frame drawing kept text wrote vertex bytes");
    assert_eq!(painter.atlas.lookups(), lookups);
    assert_eq!(pins.len(), 5);
    painter.atlas.unpin(pins);
    painter.pipeline.release(range);
    assert_eq!(painter.pipeline.retained_bytes(), 0);
}
