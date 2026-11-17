//! Kept text and icon bytes as they would be built now, from where the
//! atlas holds their glyphs and masks, with no lookup counted, nothing
//! added and nothing marked used: what a debug build compares kept
//! bytes with (IU4 `docs/plans/iu4.md` section 5).

use crate::atlas::GlyphAtlas;
use crate::atlas_types::{AtlasGlyph, AtlasKey, GlyphKey};
use crate::draw::rowed_vertex_bytes;
use crate::draw_icon::{IconDraw, icon_floats, strokes_of};
use crate::pipeline::TextPipeline;
use crate::quads::{TextDraw, glyph_floats, glyphs_of, vertices_per_draw};
use crate::raster::GlyphRequest;

impl TextPipeline {
    /// The bytes [`TextPipeline::text_bytes`] would build for `texts`
    /// now; `None` when the atlas does not hold one of their glyphs.
    #[must_use]
    pub fn placed_text_bytes(atlas: &GlyphAtlas, texts: &[TextDraw<'_>]) -> Option<Vec<u8>> {
        Self::placed_text_bytes_in(atlas, texts, (0, &[]))
    }

    /// The bytes [`TextPipeline::text_bytes_in`] would build for `texts`
    /// in frame entry `frame` and `rows` now; `None` as
    /// [`TextPipeline::placed_text_bytes`].
    #[must_use]
    pub fn placed_text_bytes_in(
        atlas: &GlyphAtlas,
        texts: &[TextDraw<'_>],
        (frame, rows): (u32, &[u32]),
    ) -> Option<Vec<u8>> {
        let glyphs = || texts.iter().flat_map(|text| glyphs_of(*text));
        let placed: Vec<AtlasGlyph> = glyphs()
            .map(|(_, r)| atlas.held(&r))
            .collect::<Option<_>>()?;
        let page = atlas.coverage_page().size();
        let floats = glyph_floats(glyphs().map(|(at, _)| at).zip(&placed), page);
        let entries = texts.iter().map(|text| glyphs_of(*text).count());
        let vertices = vertices_per_draw(&placed, entries);
        Some(rowed_vertex_bytes(&floats, (&vertices, frame, rows)))
    }

    /// The bytes [`TextPipeline::icon_bytes`] would build for `icons`
    /// now; `None` when the atlas does not hold one of their masks.
    #[must_use]
    pub fn placed_icon_bytes(atlas: &GlyphAtlas, icons: &[IconDraw<'_>]) -> Option<Vec<u8>> {
        Self::placed_icon_bytes_in(atlas, icons, (0, &[]))
    }

    /// The bytes [`TextPipeline::icon_bytes_in`] would build for `icons`
    /// in frame entry `frame` and `rows` now; `None` as
    /// [`TextPipeline::placed_icon_bytes`].
    #[must_use]
    pub fn placed_icon_bytes_in(
        atlas: &GlyphAtlas,
        icons: &[IconDraw<'_>],
        (frame, rows): (u32, &[u32]),
    ) -> Option<Vec<u8>> {
        let held = |(draw, stroke): (&IconDraw<'_>, usize)| {
            let key = AtlasKey::Icon {
                bits: std::sync::Arc::clone(draw.icon.bits(stroke)),
                size: draw.size,
            };
            atlas.entries.get(&key).map(|entry| entry.glyph)
        };
        let placed: Vec<AtlasGlyph> = strokes_of(icons).map(held).collect::<Option<_>>()?;
        let page = atlas.coverage_page().size();
        let floats = icon_floats(strokes_of(icons).map(|(draw, _)| draw).zip(&placed), page);
        let vertices = vertices_per_draw(&placed, icons.iter().map(|draw| draw.icon.strokes()));
        Some(rowed_vertex_bytes(&floats, (&vertices, frame, rows)))
    }
}

impl GlyphAtlas {
    /// Where `request` is held, as [`GlyphAtlas::get`] answers when it
    /// is: `.notdef` for a glyph refused as too large.
    fn held(&self, request: &GlyphRequest<'_>) -> Option<AtlasGlyph> {
        let key = GlyphKey::of(request);
        if self.too_large.contains(&key) {
            let notdef = GlyphRequest { id: 0, ..*request };
            let glyph = self.held(&notdef)?;
            return Some(AtlasGlyph {
                notdef: true,
                ..glyph
            });
        }
        let entry = self.entries.get(&AtlasKey::Glyph(key));
        entry.map(|entry| entry.glyph)
    }
}

#[cfg(test)]
#[path = "draw_kept_check_tests.rs"]
mod tests;
