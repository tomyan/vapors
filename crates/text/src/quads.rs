//! One quad per inked glyph: fetched from the atlas at its quarter-pixel
//! x step with the baseline rounded to a whole pixel, then mapped to
//! clip space with its atlas coordinates and colour.

use vapors_scene::{Color, Size};

use crate::atlas::{AtlasError, AtlasGlyph, GlyphAtlas};
use crate::layout::Layout;
use crate::layout_types::PlacedGlyph;
use crate::raster::{GlyphRequest, Hinting, subpixel_of};

/// One laid-out text to draw: its layout and where its origin goes
/// (device pixels, y down).
#[allow(missing_debug_implementations)] // `Layout` has no Debug
#[derive(Clone, Copy)]
pub struct TextDraw<'a> {
    /// The layout.
    pub layout: &'a Layout,
    /// Its origin.
    pub origin: [f32; 2],
}

/// Vertex floats for every inked glyph of `texts`, in order.
pub(crate) fn quads(
    atlas: &mut GlyphAtlas,
    texts: &[TextDraw<'_>],
    viewport: Size,
) -> Result<Vec<f32>, AtlasError> {
    let page = atlas.coverage_page().size();
    let glyphs = || texts.iter().flat_map(|text| glyphs_of(*text));
    let placed = placed_all(glyphs, |(_, request)| atlas.get(request))?;
    let mut floats = Vec::new();
    for ((at, _), placed) in glyphs().zip(&placed) {
        if !placed.is_empty() {
            push_glyph(&mut floats, &at, placed, page, viewport);
        }
    }
    Ok(floats)
}

/// The quad of one inked glyph placed at `at`.
fn push_glyph(floats: &mut Vec<f32>, at: &GlyphAt, placed: &AtlasGlyph, page: u32, viewport: Size) {
    #[allow(clippy::cast_precision_loss)] // pixel positions
    let top_left = [(at.x + placed.left) as f32, at.baseline + placed.top as f32];
    push_quad(floats, top_left, placed, page, viewport, at.color);
}

/// Where a glyph's bitmap goes: its whole-pixel x, its baseline rounded
/// to a whole pixel, and its colour.
struct GlyphAt {
    x: i32,
    baseline: f32,
    color: Color,
}

/// Each glyph of `text`, placed, with its atlas request.
fn glyphs_of(text: TextDraw<'_>) -> impl Iterator<Item = (GlyphAt, GlyphRequest<'_>)> {
    let hinting = Hinting::for_scale(text.layout.scale());
    let [x, y] = text.origin;
    text.layout.glyphs().map(move |glyph| {
        let (pixel_x, request) = request(&glyph, x, hinting);
        (glyph_at(pixel_x, y, &glyph), request)
    })
}

/// `glyph` at whole-pixel `pixel_x`, its baseline rounded under origin `y`.
fn glyph_at(pixel_x: i32, y: f32, glyph: &PlacedGlyph<'_>) -> GlyphAt {
    GlyphAt {
        x: pixel_x,
        baseline: (y + glyph.y).round(),
        color: glyph.color,
    }
}

/// Where each item is in the atlas once every one is placed. Placing one
/// can evict and repack, moving those placed before it, so all are
/// fetched first and their rects read in a second pass of cache hits.
pub(crate) fn placed_all<T, I: Iterator<Item = T>>(
    items: impl Fn() -> I,
    mut fetch: impl FnMut(&T) -> Result<AtlasGlyph, AtlasError>,
) -> Result<Vec<AtlasGlyph>, AtlasError> {
    for item in items() {
        fetch(&item)?;
    }
    items().map(|item| fetch(&item)).collect()
}

/// The glyph's whole-pixel x and its request at the quarter-pixel step.
fn request<'a>(
    glyph: &PlacedGlyph<'a>,
    origin_x: f32,
    hinting: Hinting,
) -> (i32, GlyphRequest<'a>) {
    let (pixel_x, subpixel) = subpixel_of(origin_x + glyph.x);
    let request = GlyphRequest {
        font: glyph.font,
        id: glyph.id,
        size: glyph.size,
        coords: glyph.coords,
        subpixel,
        hinting,
        skew: glyph.skew,
    };
    (pixel_x, request)
}

#[allow(clippy::cast_precision_loss)] // atlas and viewport sizes are far below 2^24
pub(crate) fn push_quad(
    floats: &mut Vec<f32>,
    top_left: [f32; 2],
    glyph: &AtlasGlyph,
    page: u32,
    viewport: Size,
    color: Color,
) {
    let [x, y, w, h] = glyph.rect;
    let (u0, v0) = (x as f32 / page as f32, y as f32 / page as f32);
    let (u1, v1) = ((x + w) as f32 / page as f32, (y + h) as f32 / page as f32);
    let (left, top) = (top_left[0], top_left[1]);
    let (right, bottom) = (left + w as f32, top + h as f32);
    let mut corner = |px: f32, py: f32, u: f32, v: f32| {
        let [cx, cy] = to_clip([px, py], viewport);
        floats.extend_from_slice(&[cx, cy, u, v, color.r, color.g, color.b, color.a]);
    };
    corner(left, top, u0, v0);
    corner(right, top, u1, v0);
    corner(left, bottom, u0, v1);
    corner(right, top, u1, v0);
    corner(right, bottom, u1, v1);
    corner(left, bottom, u0, v1);
}

/// Viewport pixels (origin top-left, y down) to clip space (-1..1, y up).
#[allow(clippy::cast_precision_loss)] // pixel counts are far below 2^24
fn to_clip(point: [f32; 2], viewport: Size) -> [f32; 2] {
    [
        point[0] / viewport.width as f32 * 2.0 - 1.0,
        1.0 - point[1] / viewport.height as f32 * 2.0,
    ]
}

#[cfg(test)]
#[path = "quads_tests.rs"]
mod tests;
