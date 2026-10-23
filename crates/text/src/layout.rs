//! A laid-out paragraph: Parley's shaping, bidi and line breaking over
//! the [`FontSystem`], read out as placed glyphs for the renderer.

use parley::AlignmentOptions;
use vapors_scene::Color;

use crate::font_system::FontSystem;
use crate::layout_runs::{alignment, glyph_runs, run_offset};
pub use crate::layout_types::{Align, Glyph, GlyphRun, LayoutOptions, LineInfo, PlacedGlyph};
use crate::style::{Span, Style};

/// A paragraph laid out by Parley, in device pixels.
pub struct Layout {
    inner: parley::Layout<Color>,
    text: String,
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
        for property in base.properties(options.scale) {
            builder.push_default(property);
        }
        for span in spans {
            for property in span.style.properties(options.scale) {
                builder.push(property, span.range.clone());
            }
        }
        let mut inner = builder.build(text);
        inner.break_all_lines(options.width.map(|w| w * options.scale));
        inner.align(alignment(options.align), AlignmentOptions::default());
        let runs = glyph_runs(&inner);
        Self {
            inner,
            text: text.to_owned(),
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
            top: line.metrics().block_min_coord,
            height: line.metrics().block_max_coord - line.metrics().block_min_coord,
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
                skew: run.skew,
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

    /// The text laid out.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Parley's layout, for details this type does not expose.
    #[must_use]
    pub fn inner(&self) -> &parley::Layout<Color> {
        &self.inner
    }
}

#[cfg(test)]
#[path = "layout_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "fallback_tests.rs"]
mod fallback_tests;
