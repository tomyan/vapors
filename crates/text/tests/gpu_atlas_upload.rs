//! GPU tier: a draw uploads only the part of the atlas page that changed
//! since the last upload, and the whole page when it is new or was
//! repacked; either way the pixels match a draw from a fresh atlas.

#[path = "support/painter.rs"]
mod painter;

use painter::{Painter, diagonal_icon};

/// Holds every 16 px Geist glyph used here with room to spare.
const LARGE_PAGE: u32 = 128;
/// Holds four 16 px capitals, not eight, so a second word evicts the first.
const SMALL_PAGE: u32 = 32;
/// The coverage bitmap of Geist "B" at 16 px, one byte per pixel, as the
/// atlas stored it when measured (`GlyphAtlas::get` rect width * height).
const B_GLYPH_BYTES: u64 = 120;

#[test]
fn gpu_a_new_page_is_uploaded_whole() {
    // Given
    let mut painter = Painter::new(LARGE_PAGE);

    // When
    let drawn = painter.text("A");

    // Then
    assert_eq!(drawn.atlas_bytes, u64::from(LARGE_PAGE * LARGE_PAGE));
}

#[test]
fn gpu_a_draw_adding_one_glyph_uploads_exactly_that_glyphs_bytes() {
    // Given
    let mut painter = Painter::new(LARGE_PAGE);
    painter.text("A");

    // When
    let drawn = painter.text("AB");

    // Then
    assert_eq!(drawn.atlas_bytes, B_GLYPH_BYTES);
}

#[test]
fn gpu_a_draw_adding_no_glyph_uploads_nothing() {
    // Given
    let mut painter = Painter::new(LARGE_PAGE);
    painter.text("AB");

    // When
    let drawn = painter.text("AB");

    // Then
    assert_eq!(drawn.atlas_bytes, 0);
}

#[test]
fn gpu_glyphs_added_to_a_used_page_draw_as_from_a_fresh_atlas() {
    // Given
    let mut warm = Painter::new(LARGE_PAGE);
    warm.text("Ho");
    let fresh = Painter::new(LARGE_PAGE).text("Hello");

    // When
    let drawn = warm.text("Hello");

    // Then
    assert!(drawn.atlas_bytes < u64::from(LARGE_PAGE * LARGE_PAGE));
    assert!(
        drawn.pixels == fresh.pixels,
        "pixels differ from a fresh atlas"
    );
}

#[test]
fn gpu_an_icon_added_to_a_used_page_draws_as_from_a_fresh_atlas() {
    // Given
    let icon = diagonal_icon();
    let mut warm = Painter::new(LARGE_PAGE);
    warm.text("Ho");
    let fresh = Painter::new(LARGE_PAGE).icon(&icon);

    // When
    let drawn = warm.icon(&icon);

    // Then
    assert!(drawn.atlas_bytes < u64::from(LARGE_PAGE * LARGE_PAGE));
    assert!(
        drawn.pixels == fresh.pixels,
        "pixels differ from a fresh atlas"
    );
}

#[test]
fn gpu_a_repack_uploads_the_whole_page_and_draws_as_from_a_fresh_atlas() {
    // Given: the page full of last frame's glyphs
    let mut warm = Painter::new(SMALL_PAGE);
    warm.text("ABCD");
    let fresh = Painter::new(SMALL_PAGE).text("EFGH");

    // When
    let drawn = warm.text("EFGH");

    // Then
    assert!(warm.atlas.evictions() > 0, "the page was not repacked");
    assert_eq!(drawn.atlas_bytes, u64::from(SMALL_PAGE * SMALL_PAGE));
    assert!(
        drawn.pixels == fresh.pixels,
        "pixels differ from a fresh atlas"
    );
}

#[test]
fn gpu_a_repack_in_a_frame_that_already_added_a_glyph_uploads_the_whole_page() {
    // Given: three glyphs from last frame; the first draw adds a fourth,
    // so the second draw's three new glyphs only fit after a repack
    let mut warm = Painter::new(SMALL_PAGE);
    warm.text("ABC");
    let fresh = Painter::new(SMALL_PAGE).two_words("D", "EFG");

    // When
    let drawn = warm.two_words("D", "EFG");

    // Then
    assert!(warm.atlas.evictions() > 0, "the page was not repacked");
    assert_eq!(drawn.atlas_bytes, u64::from(SMALL_PAGE * SMALL_PAGE));
    assert!(
        drawn.pixels == fresh.pixels,
        "pixels differ from a fresh atlas"
    );
}
