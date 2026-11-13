//! Drawing icons: each is fetched from the atlas at its device size and
//! becomes one quad of its coverage tinted by its colour. Text's gamma
//! and contrast boost are glyph-specific (docs/icons.md, Paint), so icons
//! draw with gamma 1 and contrast 0, which the shader makes a plain blend
//! of coverage.

use vapors_scene::Color;

use crate::atlas::{AtlasError, AtlasGlyph, GlyphAtlas};
use crate::atlas_pins::AtlasPins;
use crate::draw::RenderTarget;
use crate::icon::Icon;
use crate::pipeline::{BlendParams, TextPipeline};
use crate::quads::{placed_all, push_quad};

/// The blend icons draw with: coverage times the colour's alpha.
pub const ICON_BLEND: BlendParams = BlendParams {
    gamma: 1.0,
    contrast: 0.0,
};

/// One icon to draw, in device pixels; the caller snaps its content box.
#[derive(Clone, Copy, Debug)]
pub struct IconDraw<'a> {
    /// The icon.
    pub icon: &'a Icon,
    /// Top-left of the content box.
    pub origin: [i32; 2],
    /// Width and height of the content box.
    pub size: [u32; 2],
    /// The element's `color`.
    pub colour: Color,
}

impl TextPipeline {
    /// Draw `icons` into `target`, one quad per path: overlapping paths of
    /// a translucent colour compound as separate paints, as in Chromium.
    ///
    /// # Errors
    ///
    /// [`AtlasError`] when an icon does not fit a page or the frame's
    /// glyphs and icons overflow one; nothing is drawn then.
    ///
    /// # Panics
    ///
    /// If `atlas`'s page is smaller than this pipeline's texture.
    pub fn draw_icons(
        &mut self,
        target: &RenderTarget<'_>,
        atlas: &mut GlyphAtlas,
        icons: &[IconDraw<'_>],
    ) -> Result<(), AtlasError> {
        let floats = icon_quads(atlas, icons, None)?;
        self.record(target, atlas, &floats, ICON_BLEND);
        Ok(())
    }
}

/// Vertex floats for every inked icon of `icons`, each mask pinned into
/// `pins` when given.
pub(crate) fn icon_quads(
    atlas: &mut GlyphAtlas,
    icons: &[IconDraw<'_>],
    pins: Option<&mut AtlasPins>,
) -> Result<Vec<f32>, AtlasError> {
    let strokes = || strokes_of(icons);
    let fetch = |&(draw, i): &(&IconDraw<'_>, usize), pins: Option<&mut AtlasPins>| {
        atlas.icon_pinned((draw.icon, i, draw.size), pins)
    };
    let placed = placed_all(strokes, pins, fetch)?;
    // read after placing: placing may grow the page
    let page = atlas.coverage_page().size();
    Ok(icon_floats(
        strokes().map(|(draw, _)| draw).zip(&placed),
        page,
    ))
}

/// Each path of each of `icons`, by its index in its icon.
pub(crate) fn strokes_of<'a, 'b>(
    icons: &'a [IconDraw<'b>],
) -> impl Iterator<Item = (&'a IconDraw<'b>, usize)> {
    icons
        .iter()
        .flat_map(|d| (0..d.icon.strokes()).map(move |i| (d, i)))
}

/// Vertex floats for each inked mask of `placed`, over its icon's box,
/// on a page of `page` pixels.
pub(crate) fn icon_floats<'a, 'b: 'a>(
    placed: impl Iterator<Item = (&'a IconDraw<'b>, &'a AtlasGlyph)>,
    page: u32,
) -> Vec<f32> {
    let mut floats = Vec::new();
    for (draw, placed) in placed.filter(|(_, placed)| !placed.is_empty()) {
        #[allow(clippy::cast_precision_loss)] // pixel positions
        let top_left = draw.origin.map(|n| n as f32);
        push_quad(&mut floats, top_left, placed, page, draw.colour);
    }
    floats
}

#[cfg(test)]
#[path = "draw_icon_tests.rs"]
mod tests;
