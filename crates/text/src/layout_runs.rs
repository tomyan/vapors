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

pub(crate) fn glyph_runs(layout: &parley::Layout<Color>) -> Vec<GlyphRun> {
    layout
        .lines()
        .flat_map(|line| {
            line.items().filter_map(|item| match item {
                parley::PositionedLayoutItem::GlyphRun(run) => Some(glyph_run(&run)),
                parley::PositionedLayoutItem::InlineBox(_) => None,
            })
        })
        .collect()
}

fn glyph_run(run: &parley::GlyphRun<'_, Color>) -> GlyphRun {
    let inner = run.run();
    GlyphRun {
        font: inner.font().clone(),
        size: inner.font_size(),
        coords: inner.normalized_coords().to_vec(),
        color: run.style().brush,
        glyphs: run
            .positioned_glyphs()
            .map(|g| Glyph {
                id: g.id,
                x: g.x,
                y: g.y,
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
