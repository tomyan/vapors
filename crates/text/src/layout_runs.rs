//! Reading Parley's layout out as glyph runs, and the small
//! conversions between Parley's types and this crate's.

use parley::Alignment;
use vapors_scene::Color;

use crate::baseline::{BaselineRule, line_shift};
use crate::layout_types::{Align, Glyph, GlyphRun};

pub(crate) fn run_offset(line: &parley::Line<'_, Color>, run: &parley::Run<'_, Color>) -> f32 {
    line.runs()
        .take_while(|r| r.index() != run.index())
        .map(|r| r.advance())
        .sum()
}

/// Each line's glyph runs, moved onto the baseline `rule` gives the line
/// (`baseline.rs`); left on Parley's when the rule is unresolvable.
pub(crate) fn glyph_runs(layout: &parley::Layout<Color>, rule: &BaselineRule) -> Vec<GlyphRun> {
    layout
        .lines()
        .flat_map(|line| {
            let shift = line_shift(&line, rule, 1.0).unwrap_or(0.0);
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
