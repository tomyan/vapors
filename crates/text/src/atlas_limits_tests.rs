//! What the atlas does at its limits: a glyph larger than a page, a
//! .notdef larger than a page, a frame that cannot fit.

use std::sync::Arc;

use parley::FontData;
use parley::fontique::Blob;
use skrifa::MetadataProvider;

use super::*;
use crate::fonts::GEIST;
use crate::raster::{GlyphRequest, Hinting};

fn geist() -> FontData {
    FontData::new(Blob::new(Arc::new(GEIST)), 0)
}

fn glyph_id(font: &FontData, c: char) -> u32 {
    let font_ref = skrifa::FontRef::new(font.data.as_ref()).expect("font");
    font_ref.charmap().map(c).expect("mapped").to_u32()
}

fn request(font: &FontData, c: char, subpixel: u8) -> GlyphRequest<'_> {
    request_at(font, c, subpixel, 16.0)
}

fn request_at(font: &FontData, c: char, subpixel: u8, size: f32) -> GlyphRequest<'_> {
    GlyphRequest {
        font,
        id: glyph_id(font, c),
        size,
        coords: &[],
        subpixel,
        hinting: Hinting::Light,
        skew: 0.0,
    }
}

fn atlas(page_size: u32) -> GlyphAtlas {
    GlyphAtlas::new(AtlasConfig { page_size })
}

#[test]
fn a_glyph_larger_than_the_page_is_refused_once_and_drawn_as_notdef() {
    // Given: a 128 px page; W at 150 px is wider than it, .notdef is not
    let font = geist();
    let mut atlas = atlas(128);

    // When
    let first = atlas
        .get(&request_at(&font, 'W', 0, 150.0))
        .expect("notdef stands in");
    let rasterised = atlas.rasterisations();
    let second = atlas
        .get(&request_at(&font, 'W', 0, 150.0))
        .expect("still notdef");

    // Then
    assert!(first.notdef && !first.is_empty());
    assert_eq!(first, second);
    assert_eq!(atlas.too_large(), 1);
    assert_eq!(atlas.rasterisations(), rasterised, "not attempted again");
}

#[test]
fn a_notdef_larger_than_the_page_is_an_error_naming_glyph_size_and_page() {
    // Given
    let font = geist();
    let mut atlas = atlas(32);

    // When
    let error = atlas
        .get(&request_at(&font, 'W', 0, 400.0))
        .expect_err("nothing can stand in");

    // Then
    let message = error.to_string();
    assert!(message.contains("glyph 0 at 400 px"), "{message}");
    assert!(message.contains("32x32"), "{message}");
    assert!(message.contains("Fix:"), "{message}");
}

#[test]
fn a_frame_that_cannot_fit_in_the_page_is_refused_with_the_fix() {
    // Given
    let font = geist();
    let mut atlas = atlas(16);
    atlas.begin_frame();

    // When
    let result = ('a'..='z')
        .map(|c| atlas.get(&request(&font, c, 0)))
        .find(Result::is_err)
        .expect("overflows");

    // Then
    let message = result.expect_err("error").to_string();
    assert!(message.contains("16"), "{message}");
    assert!(message.contains("Fix:"), "{message}");
}
