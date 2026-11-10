//! Drawing icons: each is fetched from the atlas at its device size and
//! becomes one quad of its coverage tinted by its colour. Text's gamma
//! and contrast boost are glyph-specific (docs/icons.md, Paint), so icons
//! draw with gamma 1 and contrast 0, which the shader makes a plain blend
//! of coverage.

use vapors_scene::{Color, Size};

use crate::atlas::{AtlasError, GlyphAtlas};
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
    /// Draw `icons` into `target`, each as one quad.
    ///
    /// # Errors
    ///
    /// [`AtlasError`] when an icon does not fit a page or the frame's
    /// glyphs and icons overflow one; nothing is drawn then.
    ///
    /// # Panics
    ///
    /// If `atlas`'s page is not the size this pipeline's texture was made for.
    pub fn draw_icons(
        &mut self,
        target: &RenderTarget<'_>,
        atlas: &mut GlyphAtlas,
        icons: &[IconDraw<'_>],
    ) -> Result<(), AtlasError> {
        let floats = icon_quads(atlas, icons, target.viewport)?;
        self.submit(target, atlas, &floats, ICON_BLEND);
        Ok(())
    }
}

/// Vertex floats for every inked icon of `icons`.
pub(crate) fn icon_quads(
    atlas: &mut GlyphAtlas,
    icons: &[IconDraw<'_>],
    viewport: Size,
) -> Result<Vec<f32>, AtlasError> {
    let page = atlas.coverage_page().size();
    let placed = placed_all(|| icons.iter(), |draw| atlas.icon(draw.icon, draw.size))?;
    let mut floats = Vec::new();
    for (draw, placed) in icons.iter().zip(&placed) {
        if placed.is_empty() {
            continue;
        }
        #[allow(clippy::cast_precision_loss)] // pixel positions
        let top_left = draw.origin.map(|n| n as f32);
        push_quad(&mut floats, top_left, placed, page, viewport, draw.colour);
    }
    Ok(floats)
}

#[cfg(test)]
#[path = "draw_icon_tests.rs"]
mod tests;
