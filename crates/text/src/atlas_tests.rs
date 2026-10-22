//! The atlas caches rasterised glyphs, evicts the ones a frame did not
//! use when a page fills, and counts both.

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
    GlyphRequest {
        font,
        id: glyph_id(font, c),
        size: 16.0,
        coords: &[],
        subpixel,
        hinting: Hinting::Light,
    }
}

fn atlas(page_size: u32) -> GlyphAtlas {
    GlyphAtlas::new(AtlasConfig { page_size })
}

#[test]
fn a_glyph_is_rasterised_once_and_the_entry_reused() {
    // Given
    let font = geist();
    let mut atlas = atlas(64);

    // When
    let first = atlas.get(&request(&font, 'a', 0)).expect("ok");
    let second = atlas.get(&request(&font, 'a', 0)).expect("ok");

    // Then
    assert_eq!(first, second);
    assert_eq!(atlas.rasterisations(), 1);
    assert_eq!(atlas.evictions(), 0);
}

#[test]
fn each_subpixel_offset_is_its_own_entry() {
    // Given
    let font = geist();
    let mut atlas = atlas(64);

    // When
    let at_zero = atlas.get(&request(&font, 'a', 0)).expect("ok");
    let at_one = atlas.get(&request(&font, 'a', 1)).expect("ok");

    // Then
    assert_ne!(at_zero.rect, at_one.rect);
    assert_eq!(atlas.rasterisations(), 2);
}

#[test]
fn the_page_holds_the_coverage_at_the_returned_rect() {
    // Given
    let font = geist();
    let mut atlas = atlas(64);
    let expected = Rasterizer::new()
        .rasterize(&request(&font, 'a', 0))
        .expect("ok");

    // When
    let entry = atlas.get(&request(&font, 'a', 0)).expect("ok");
    let page = atlas.coverage_page();
    let [x, y, w, h] = entry.rect;
    let mut stored = Vec::new();
    for row in 0..h {
        let start = ((y + row) * page.size() + x) as usize;
        stored.extend_from_slice(&page.pixels()[start..start + w as usize]);
    }

    // Then
    assert_eq!(entry.left, expected.left);
    assert_eq!(entry.top, expected.top);
    assert_eq!(stored, expected.data);
    assert!(page.dirty());
}

#[test]
fn a_space_takes_no_room_in_the_page() {
    // Given
    let font = geist();
    let mut atlas = atlas(64);

    // When
    let entry = atlas.get(&request(&font, ' ', 0)).expect("ok");

    // Then
    assert!(entry.is_empty());
    assert_eq!(entry.rect[2..], [0, 0]);
}

#[test]
fn a_full_page_evicts_glyphs_unused_this_frame_and_they_rasterise_again() {
    // Given: a page too small for the whole alphabet at 16 px
    let font = geist();
    let mut atlas = atlas(32);
    atlas.begin_frame();
    atlas.get(&request(&font, 'a', 0)).expect("ok");
    let after_a = atlas.rasterisations();

    // When: later frames each need one other glyph, until the page overflows
    for c in 'b'..='z' {
        atlas.begin_frame();
        atlas
            .get(&request(&font, c, 0))
            .expect("fits after eviction");
    }
    let evicted = atlas.evictions();
    atlas.begin_frame();
    atlas.get(&request(&font, 'a', 0)).expect("ok");

    // Then
    assert!(evicted > 0, "nothing was evicted");
    assert!(atlas.rasterisations() > after_a + 25, "a rasterised again");
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
