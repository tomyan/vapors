//! Where a layout's glyphs can ink, for culling: each run's glyph
//! positions widened by its font's bounding box (the `head` table's
//! union of every glyph's outline), its slant and margins. Wider than
//! the ink, never narrower: an item culled by it must change no pixel.

use skrifa::raw::TableProvider;
use vapors_scene::Rect;

use crate::layout_types::GlyphRun;

/// Extra reach per side, as a share of the font size: `head`'s box is
/// the default instance's, and another instance (a heavier weight) can
/// draw a little past it.
const VARIATION_MARGIN: f32 = 0.25;

/// Pixels per side for hinting, which moves outline points by under a
/// pixel, and for the coverage's antialiased edge.
const RASTER_MARGIN: f32 = 2.0;

/// Ink of a run whose font has no readable bounding box: everywhere, so
/// it is never culled.
const UNBOUNDED: Rect = Rect {
    left: f32::NEG_INFINITY,
    top: f32::NEG_INFINITY,
    right: f32::INFINITY,
    bottom: f32::INFINITY,
};

/// Every pixel `runs` can ink, device pixels from the layout origin;
/// an empty rectangle at the origin when they have no glyphs.
pub(crate) fn ink(runs: &[GlyphRun]) -> Rect {
    runs.iter()
        .filter_map(run_ink)
        .reduce(union)
        .unwrap_or_default()
}

/// `run`'s ink; `None` when it has no glyphs.
fn run_ink(run: &GlyphRun) -> Option<Rect> {
    let pens = pen_span(run)?;
    let Some([x_min, y_min, x_max, y_max]) = font_box(run) else {
        return Some(UNBOUNDED);
    };
    let slant = run.skew.abs() * y_max.abs().max(y_min.abs());
    let margin = run.size * VARIATION_MARGIN + RASTER_MARGIN;
    let ink = Rect::new(
        pens.left + x_min - slant,
        pens.top - y_max,
        pens.right + x_max + slant,
        pens.bottom - y_min,
    );
    Some(ink.grown(margin))
}

/// The span of `run`'s glyph origins: x from the first to the last
/// pen position, y over the baselines. `None` without glyphs.
fn pen_span(run: &GlyphRun) -> Option<Rect> {
    let first = run.glyphs.first()?;
    let start = Rect::new(first.x, first.y, first.x, first.y);
    Some(run.glyphs.iter().fold(start, |span, glyph| {
        union(span, Rect::new(glyph.x, glyph.y, glyph.x, glyph.y))
    }))
}

/// The font's `head` box at the run's size, y up: `[x_min, y_min,
/// x_max, y_max]`; `None` when the font or its `head` cannot be read.
fn font_box(run: &GlyphRun) -> Option<[f32; 4]> {
    let font = skrifa::FontRef::from_index(run.font.data.as_ref(), run.font.index).ok()?;
    let head = font.head().ok()?;
    let units_per_em = head.units_per_em();
    if units_per_em == 0 {
        return None;
    }
    let scale = run.size / f32::from(units_per_em);
    let [x_min, y_min, x_max, y_max] = [head.x_min(), head.y_min(), head.x_max(), head.y_max()];
    Some([x_min, y_min, x_max, y_max].map(|v| f32::from(v) * scale))
}

fn union(a: Rect, b: Rect) -> Rect {
    Rect::new(
        a.left.min(b.left),
        a.top.min(b.top),
        a.right.max(b.right),
        a.bottom.max(b.bottom),
    )
}

#[cfg(test)]
#[path = "layout_ink_tests.rs"]
mod tests;
