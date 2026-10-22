//! A laid-out paragraph: Parley's shaping, bidi and line breaking over
//! the [`FontSystem`], read out as placed glyphs for the renderer.

use std::ops::Range;

use parlance::NormalizedCoord;
use parley::{Alignment, AlignmentOptions, FontData};
use vapors_scene::Color;

use crate::font_system::FontSystem;
use crate::style::{Span, Style};

/// Horizontal alignment of lines inside the layout width.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Align {
    /// The writing direction's start edge.
    #[default]
    Start,
    /// The writing direction's end edge.
    End,
    /// Centred.
    Center,
    /// Stretched to the width, except the last line.
    Justify,
}

/// Width, alignment and display scale for a layout.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutOptions {
    /// Wrap width in logical pixels; `None` never wraps.
    pub width: Option<f32>,
    /// Line alignment inside the width.
    pub align: Align,
    /// Device pixels per logical pixel; the layout is in device pixels.
    pub scale: f32,
}

impl Default for LayoutOptions {
    fn default() -> Self {
        Self {
            width: None,
            align: Align::Start,
            scale: 1.0,
        }
    }
}

/// One line of a layout, in device pixels from the layout origin.
#[derive(Clone, Debug, PartialEq)]
pub struct LineInfo {
    /// The bytes of the text on this line, trailing whitespace included.
    pub text_range: Range<usize>,
    /// Baseline y, unsnapped.
    pub baseline: f32,
    /// Inked width.
    pub advance: f32,
}

/// One glyph of a [`GlyphRun`]. Positions are device pixels from the
/// layout origin; `y` is the baseline.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Glyph {
    /// Glyph id in the run's font.
    pub id: u32,
    /// Left edge of the glyph's advance box.
    pub x: f32,
    /// Baseline.
    pub y: f32,
    /// Horizontal advance.
    pub advance: f32,
}

/// Consecutive glyphs in one font, size, variation and colour.
#[derive(Clone, Debug)]
pub struct GlyphRun {
    /// The font the glyphs are in.
    pub font: FontData,
    /// Font size in device pixels.
    pub size: f32,
    /// Variation coordinates the run was shaped with.
    pub coords: Vec<NormalizedCoord>,
    /// Ink colour.
    pub color: Color,
    /// The glyphs, in visual order.
    pub glyphs: Vec<Glyph>,
}

/// A glyph with its run's font, size, variation and colour beside it.
#[derive(Clone, Copy, Debug)]
pub struct PlacedGlyph<'a> {
    /// The font the glyph is in.
    pub font: &'a FontData,
    /// Glyph id in that font.
    pub id: u32,
    /// Left edge of the glyph's advance box.
    pub x: f32,
    /// Baseline.
    pub y: f32,
    /// Horizontal advance.
    pub advance: f32,
    /// Font size in device pixels.
    pub size: f32,
    /// Variation coordinates the run was shaped with.
    pub coords: &'a [NormalizedCoord],
    /// Ink colour.
    pub color: Color,
}

/// A paragraph laid out by Parley, in device pixels.
pub struct Layout {
    inner: parley::Layout<Color>,
    scale: f32,
    runs: Vec<GlyphRun>,
}

impl Layout {
    /// Lay out `text` in one style.
    #[must_use]
    pub fn new(
        system: &mut FontSystem,
        text: &str,
        style: &Style<'_>,
        options: &LayoutOptions,
    ) -> Self {
        Self::styled(system, text, style, &[], options)
    }

    /// Lay out `text` in `base`, with `spans` overriding ranges of it.
    #[must_use]
    pub fn styled(
        system: &mut FontSystem,
        text: &str,
        base: &Style<'_>,
        spans: &[Span<'_>],
        options: &LayoutOptions,
    ) -> Self {
        let mut builder = system.ranged_builder(text, options.scale);
        for property in base.properties() {
            builder.push_default(property);
        }
        for span in spans {
            for property in span.style.properties() {
                builder.push(property, span.range.clone());
            }
        }
        let mut inner = builder.build(text);
        inner.break_all_lines(options.width.map(|w| w * options.scale));
        inner.align(alignment(options.align), AlignmentOptions::default());
        let runs = glyph_runs(&inner);
        Self {
            inner,
            scale: options.scale,
            runs,
        }
    }

    /// Device pixels per logical pixel this was laid out at.
    #[must_use]
    pub fn scale(&self) -> f32 {
        self.scale
    }

    /// Widest line, in device pixels.
    #[must_use]
    pub fn width(&self) -> f32 {
        self.inner.width()
    }

    /// Height of all lines, in device pixels.
    #[must_use]
    pub fn height(&self) -> f32 {
        self.inner.height()
    }

    /// The lines, top to bottom.
    pub fn lines(&self) -> impl Iterator<Item = LineInfo> + '_ {
        self.inner.lines().map(|line| LineInfo {
            text_range: line.text_range(),
            baseline: line.metrics().baseline,
            advance: line.metrics().advance,
        })
    }

    /// The glyph runs, line by line, in visual order.
    #[must_use]
    pub fn runs(&self) -> &[GlyphRun] {
        &self.runs
    }

    /// Every glyph in visual order, line by line, with its run's details.
    pub fn glyphs(&self) -> impl Iterator<Item = PlacedGlyph<'_>> + '_ {
        self.runs.iter().flat_map(|run| {
            run.glyphs.iter().map(move |g| PlacedGlyph {
                font: &run.font,
                id: g.id,
                x: g.x,
                y: g.y,
                advance: g.advance,
                size: run.size,
                coords: &run.coords,
                color: run.color,
            })
        })
    }

    /// Left edge of the cluster containing byte `index`, in device pixels.
    ///
    /// # Panics
    ///
    /// If `index` is outside the text; that is a caller bug and the
    /// message says which byte.
    #[must_use]
    pub fn cluster_x(&self, index: usize) -> f32 {
        let cluster = parley::Cluster::from_byte_index(&self.inner, index)
            .unwrap_or_else(|| panic!("byte {index} is not in the layout's text"));
        let line = cluster.line();
        let run = cluster.run();
        let before: f32 = run
            .visual_clusters()
            .take_while(|c| c.text_range() != cluster.text_range())
            .map(|c| c.advance())
            .sum();
        line.metrics().offset + run_offset(&line, &run) + before
    }

    /// Parley's layout, for details this type does not expose.
    #[must_use]
    pub fn inner(&self) -> &parley::Layout<Color> {
        &self.inner
    }
}

fn run_offset(line: &parley::Line<'_, Color>, run: &parley::Run<'_, Color>) -> f32 {
    line.runs()
        .take_while(|r| r.index() != run.index())
        .map(|r| r.advance())
        .sum()
}

fn glyph_runs(layout: &parley::Layout<Color>) -> Vec<GlyphRun> {
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

fn alignment(align: Align) -> Alignment {
    match align {
        Align::Start => Alignment::Start,
        Align::End => Alignment::End,
        Align::Center => Alignment::Center,
        Align::Justify => Alignment::Justify,
    }
}

#[cfg(test)]
#[path = "layout_tests.rs"]
mod tests;
