//! GPU tier: text vertices kept between frames (IU4 PR 3) draw the
//! pixels the immediate draw does, and a frame drawing them again
//! writes no vertex bytes and looks nothing up in the atlas; drawn
//! under a scroll frame's shift (IU4 4c), they land where the text
//! drawn at the moved origin does.

#[path = "support/painter.rs"]
mod painter;

use std::cell::RefCell;

use painter::Painter;
use vapors_renderer::{KeptRange, RowOffsets, Shift};
use vapors_text::{
    AtlasConfig, AtlasPins, BlendParams, Family, Layout, LayoutOptions, Style, TextDraw,
    TextPipeline,
};

const PAGE: u32 = 128;
/// No scroll frame's shift.
const NONE: Shift = Shift::NONE;

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
        pipeline.draw_kept(target, atlas, (&range, &pins), BlendParams::default(), NONE);
        *kept.borrow_mut() = Some((range, pins));
        Ok(())
    });
    let (range, pins) = kept.into_inner().expect("kept in the frame");
    (drawn.pixels, range, pins)
}

/// A later frame drawing kept bytes again: its pixels and the vertex
/// bytes it wrote.
fn draw_again(painter: &mut Painter, kept: (&KeptRange, &AtlasPins)) -> (Vec<u8>, u64) {
    let written = RefCell::new(0);
    let drawn = painter.frame(|pipeline, atlas, target| {
        pipeline.draw_kept(target, atlas, kept, BlendParams::default(), NONE);
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
    let (again, written) = draw_again(&mut painter, (&range, &pins));

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

#[test]
#[should_panic(expected = "kept bytes built at atlas generation")]
fn gpu_kept_text_drawn_after_the_atlas_grew_and_repacked_panics() {
    // Given: "Hello" kept in a 64 px page that can grow
    let config = AtlasConfig {
        page_size: 64,
        max_page_size: 256,
    };
    let mut painter = Painter::with_atlas(config);
    let (_, range, pins) = keep_and_draw(&mut painter, "Hello");
    let style = Style::new(Family::Geist, 48.0);
    let large = Layout::new(&mut painter.system, "WM", &style, &LayoutOptions::default());

    // When: large glyphs grow the page, which repacks it, then "Hello"
    // is drawn from its bytes
    painter.frame(|pipeline, atlas, target| {
        pipeline.draw(target, atlas, &large, [0.0, 60.0], BlendParams::default())?;
        pipeline.draw_kept(target, atlas, (&range, &pins), BlendParams::default(), NONE);
        Ok(())
    });
}

#[test]
fn gpu_kept_text_drawn_with_a_shift_matches_the_text_drawn_at_the_moved_origin() {
    // Given: "Hello" drawn at 5, 9
    let mut immediate = Painter::new(PAGE);
    let layout = immediate.layout("Hello");
    let moved = immediate.frame(|pipeline, atlas, target| {
        pipeline.draw(target, atlas, &layout, [5.0, 9.0], BlendParams::default())
    });
    let mut painter = Painter::new(PAGE);
    let (_, range, pins) = keep_and_draw(&mut painter, "Hello");

    // When: its bytes kept at 2, 4 drawn shifted by 3, 5
    let shifted = painter.frame(|pipeline, atlas, target| {
        let params = BlendParams::default();
        pipeline.draw_kept(
            target,
            atlas,
            (&range, &pins),
            params,
            Shift::whole([3.0, 5.0]),
        );
        Ok(())
    });

    // Then
    assert!(shifted.pixels == moved.pixels, "shifted kept text differs");
    painter.atlas.unpin(pins);
    painter.pipeline.release(range);
}

#[test]
fn gpu_kept_text_drawn_in_a_row_matches_the_text_drawn_at_the_rows_offset() {
    // Given: "Hello" drawn at 5, 19
    let mut immediate = Painter::new(PAGE);
    let layout = immediate.layout("Hello");
    let moved = immediate.frame(|pipeline, atlas, target| {
        pipeline.draw(target, atlas, &layout, [5.0, 19.0], BlendParams::default())
    });
    let mut painter = Painter::new(PAGE);
    let (_, range, pins) = keep_and_draw(&mut painter, "Hello");

    // When: its bytes kept at 2, 4 drawn shifted by 3, 5, in row 0 of a
    // lane, ten pixels down
    let rowed = painter.frame(|pipeline, atlas, target| {
        let rows = RowOffsets::whole(&[[0.0, 10.0]]);
        let moved = (Shift::whole([3.0, 5.0]), &rows);
        pipeline.draw_kept_in_rows(
            target,
            atlas,
            (&range, &pins),
            (BlendParams::default(), moved),
        );
        Ok(())
    });

    // Then
    assert!(rowed.pixels == moved.pixels, "kept text in a row differs");
    painter.atlas.unpin(pins);
    painter.pipeline.release(range);
}

#[test]
fn gpu_texts_built_in_two_rows_draw_each_at_its_rows_offset() {
    // Given: "Hi" drawn at 2, 4 and "Yo" at 2, 34
    let mut immediate = Painter::new(PAGE);
    let (hi, yo) = (immediate.layout("Hi"), immediate.layout("Yo"));
    let apart = immediate.frame(|pipeline, atlas, target| {
        pipeline.draw(target, atlas, &hi, [2.0, 4.0], BlendParams::default())?;
        pipeline.draw(target, atlas, &yo, [2.0, 34.0], BlendParams::default())
    });

    // When: both built at 2, 4, "Yo" in row 1, thirty pixels down
    let mut painter = Painter::new(PAGE);
    let (hi, yo) = (painter.layout("Hi"), painter.layout("Yo"));
    let kept = RefCell::new(None);
    let rowed = painter.frame(|pipeline, atlas, target| {
        let at = |layout| TextDraw {
            layout,
            origin: [2.0, 4.0],
        };
        let (bytes, pins) = TextPipeline::text_bytes_in_rows(atlas, &[at(&hi), at(&yo)], &[0, 1])?;
        let range = pipeline.keep(target, &bytes).expect("kept");
        let rows = RowOffsets::whole(&[[0.0, 0.0], [0.0, 30.0]]);
        let moved = (BlendParams::default(), (NONE, &rows));
        pipeline.draw_kept_in_rows(target, atlas, (&range, &pins), moved);
        *kept.borrow_mut() = Some((range, pins));
        Ok(())
    });

    // Then
    assert!(rowed.pixels == apart.pixels, "texts in two rows differ");
    let (range, pins) = kept.into_inner().expect("kept in the frame");
    painter.atlas.unpin(pins);
    painter.pipeline.release(range);
}
