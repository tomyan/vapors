//! Synthetic italic: a face with no italic of its own, asked for one, is
//! slanted as Skia slants it (x moves a quarter pixel right per pixel
//! up), in layout and in the rasteriser and atlas.

use std::sync::Arc;

use parley::FontData;
use parley::fontique::Blob;
use skrifa::MetadataProvider;

use crate::atlas::{AtlasConfig, GlyphAtlas};
use crate::fonts::GEIST;
use crate::raster::{GlyphRequest, Hinting, Rasterizer};
use crate::{Family, FontSystem, Layout, LayoutOptions, Style};

fn slants(italic: bool) -> Vec<f32> {
    let style = Style {
        italic,
        ..Style::new(Family::Geist, 16.0)
    };
    let layout = Layout::new(
        &mut FontSystem::bundle_only(),
        "Slant",
        &style,
        &LayoutOptions::default(),
    );
    layout.glyphs().map(|glyph| glyph.skew).collect()
}

fn request(font: &FontData, skew: f32) -> GlyphRequest<'_> {
    let id = skrifa::FontRef::new(font.data.as_ref())
        .expect("font")
        .charmap()
        .map('l')
        .expect("mapped")
        .to_u32();
    GlyphRequest {
        font,
        id,
        size: 32.0,
        coords: &[],
        subpixel: 0,
        hinting: Hinting::None,
        skew,
    }
}

#[test]
fn italic_geist_is_slanted_a_quarter_and_upright_geist_not_at_all() {
    // Then
    assert!(
        slants(true)
            .iter()
            .all(|s| (*s - 0.25).abs() < f32::EPSILON),
        "{:?}",
        slants(true)
    );
    assert!(
        slants(false).iter().all(|s| *s == 0.0),
        "{:?}",
        slants(false)
    );
}

#[test]
fn a_slanted_glyph_leans_right_over_its_height() {
    // Given: an `l`, a vertical stem about 23 px tall at 32 px
    let font = FontData::new(Blob::new(Arc::new(GEIST)), 0);
    let mut rasterizer = Rasterizer::new();

    // When
    let upright = rasterizer
        .rasterize(&request(&font, 0.0))
        .expect("rasterises");
    let slanted = rasterizer
        .rasterize(&request(&font, 0.25))
        .expect("rasterises");

    // Then: its top row starts about a quarter of its height further
    // right
    let shift = top_left_ink(&slanted) - top_left_ink(&upright);
    #[allow(clippy::cast_possible_truncation)] // a few pixels
    let expected = (f64::from(upright.height) * 0.25).round() as i32;
    assert!((shift - expected).abs() <= 1, "{shift} against {expected}");
}

/// The x of the first inked pixel in the top row, from the origin.
fn top_left_ink(coverage: &crate::raster::Coverage) -> i32 {
    let width = coverage.width as usize;
    let first = coverage.data[..width]
        .iter()
        .position(|value| *value > 0)
        .expect("ink in the top row");
    coverage.left + i32::try_from(first).expect("a narrow bitmap")
}

#[test]
fn a_slanted_glyph_is_its_own_atlas_entry() {
    // Given
    let font = FontData::new(Blob::new(Arc::new(GEIST)), 0);
    let mut atlas = GlyphAtlas::new(AtlasConfig { page_size: 128 });

    // When
    atlas.get(&request(&font, 0.0)).expect("ok");
    atlas.get(&request(&font, 0.25)).expect("ok");

    // Then
    assert_eq!(atlas.rasterisations(), 2);
}
