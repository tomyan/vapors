//! Reading Parley's layout out as glyph runs, and the small
//! conversions between Parley's types and this crate's.

use parley::Alignment;
use vapors_scene::Color;

use crate::layout_types::{Align, Glyph, GlyphRun};

pub(crate) fn run_offset(line: &parley::Line<'_, Color>, run: &parley::Run<'_, Color>) -> f32 {
    line.runs()
        .take_while(|r| r.index() != run.index())
        .map(|r| r.advance())
        .sum()
}

/// `line`'s baseline as Blink places it (`baseline_below_top`), device
/// pixels from the layout's top, in place of Parley's, which adds the
/// font's metrics unrounded and the half-leading unfloored.
pub(crate) fn baseline(line: &parley::Line<'_, Color>) -> f32 {
    let metrics = line.metrics();
    let height = metrics.block_max_coord - metrics.block_min_coord;
    // the content area is the font's ascent and descent around the baseline
    let ascent = metrics.baseline - metrics.content_block_min_coord;
    let descent = metrics.content_block_max_coord - metrics.baseline;
    metrics.block_min_coord + crate::line_metrics::baseline_below_top(height, ascent, descent)
}

pub(crate) fn glyph_runs(layout: &parley::Layout<Color>) -> Vec<GlyphRun> {
    layout
        .lines()
        .flat_map(|line| {
            let shift = baseline(&line) - line.metrics().baseline;
            line.items().filter_map(move |item| match item {
                parley::PositionedLayoutItem::GlyphRun(run) => Some(glyph_run(&run, shift)),
                parley::PositionedLayoutItem::InlineBox(_) => None,
            })
        })
        .collect()
}

/// `run`'s glyphs, moved down by `shift` onto the line's baseline.
fn glyph_run(run: &parley::GlyphRun<'_, Color>, shift: f32) -> GlyphRun {
    let inner = run.run();
    GlyphRun {
        font: inner.font().clone(),
        size: inner.font_size(),
        coords: inner.normalized_coords().to_vec(),
        skew: if inner.synthesis().skew().is_some() {
            crate::SYNTHETIC_SLANT
        } else {
            0.0
        },
        color: run.style().brush,
        glyphs: run
            .positioned_glyphs()
            .map(|g| Glyph {
                id: g.id,
                x: g.x,
                y: g.y + shift,
                advance: g.advance,
            })
            .collect(),
    }
}

pub(crate) fn alignment(align: Align) -> Alignment {
    match align {
        Align::Start => Alignment::Start,
        Align::End => Alignment::End,
        Align::Center => Alignment::Center,
        Align::Justify => Alignment::Justify,
    }
}
