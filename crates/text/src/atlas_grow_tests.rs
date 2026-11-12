//! The coverage page doubles when one frame's glyphs do not fit, keeps
//! every glyph it held, and refuses at its largest side.

use std::sync::Arc;

use parley::FontData;
use parley::fontique::Blob;
use skrifa::MetadataProvider;

use super::*;
use crate::atlas_types::{AtlasConfig, AtlasGlyph};
use crate::fonts::GEIST;
use crate::raster::{GlyphRequest, Hinting};

const LETTERS: &str = "abcdefghijklmnopqrstuvwxyz";

fn geist() -> FontData {
    FontData::new(Blob::new(Arc::new(GEIST)), 0)
}

fn request(font: &FontData, c: char) -> GlyphRequest<'_> {
    let font_ref = skrifa::FontRef::new(font.data.as_ref()).expect("font");
    GlyphRequest {
        font,
        id: font_ref.charmap().map(c).expect("mapped").to_u32(),
        size: 40.0,
        coords: &[],
        subpixel: 0,
        hinting: Hinting::Light,
        skew: 0.0,
    }
}

fn growing(page_size: u32, max_page_size: u32) -> GlyphAtlas {
    GlyphAtlas::new(AtlasConfig {
        page_size,
        max_page_size,
    })
}

/// Every letter in one frame.
fn place_letters(atlas: &mut GlyphAtlas, font: &FontData) -> Result<(), AtlasError> {
    atlas.begin_frame();
    LETTERS
        .chars()
        .try_for_each(|c| atlas.get(&request(font, c)).map(drop))
}

/// True when the page holds `entry`'s coverage at its rect.
fn page_holds(page: &AtlasPage, glyph: AtlasGlyph, data: &[u8]) -> bool {
    let [x, y, width, _] = glyph.rect;
    let side = page.size() as usize;
    data.chunks(width as usize).enumerate().all(|(row, bytes)| {
        let start = (y as usize + row) * side + x as usize;
        page.pixels()[start..start + bytes.len()] == *bytes
    })
}

#[test]
fn a_frame_that_fills_the_page_doubles_it_and_keeps_every_glyph_where_it_says() {
    // Given: 26 letters at 40 px do not fit a 64 px page
    let font = geist();
    let mut atlas = growing(64, 512);

    // When
    let placed = place_letters(&mut atlas, &font);

    // Then
    assert_eq!(placed, Ok(()));
    let side = atlas.coverage_page().size();
    assert!([128, 256, 512].contains(&side), "page is {side} px");
    assert_eq!(atlas.entries.len(), LETTERS.len());
    for entry in atlas.entries.values() {
        let [x, y, w, h] = entry.glyph.rect;
        assert!(
            x + w <= side && y + h <= side,
            "{:?} in {side}",
            entry.glyph
        );
        assert!(page_holds(
            atlas.coverage_page(),
            entry.glyph,
            &entry.coverage.data
        ));
    }
}

#[test]
fn a_frame_that_overflows_the_largest_page_is_refused_naming_it() {
    // Given
    let font = geist();
    let mut atlas = growing(64, 128);

    // When
    let placed = place_letters(&mut atlas, &font);

    // Then
    assert!(
        matches!(placed, Err(AtlasError::Full { page_size: 128, .. })),
        "{placed:?}"
    );
}

#[test]
fn the_page_does_not_grow_while_evicting_older_frames_makes_room() {
    // Given
    let font = geist();
    let mut atlas = growing(64, 512);

    // When: one letter per frame
    for c in LETTERS.chars() {
        atlas.begin_frame();
        atlas.get(&request(&font, c)).expect("fits after eviction");
    }

    // Then
    assert_eq!(atlas.coverage_page().size(), 64);
    assert!(atlas.evictions() > 0);
}

#[test]
fn a_glyph_larger_than_the_first_page_but_not_the_largest_is_placed() {
    // Given
    let font = geist();
    let mut atlas = growing(64, 512);
    let big = GlyphRequest {
        size: 120.0,
        ..request(&font, 'W')
    };

    // When
    atlas.begin_frame();
    let placed = atlas.get(&big).expect("placed");

    // Then
    assert!(!placed.notdef, "{placed:?}");
    assert!(atlas.coverage_page().size() > 64);
}
