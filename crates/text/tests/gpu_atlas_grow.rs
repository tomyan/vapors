//! GPU tier: when a frame's glyphs do not fit the coverage page, the
//! page grows, the text pipeline makes a texture of the new size, and
//! the pixels match a draw from a page that was large enough from the
//! start.

#[path = "support/painter.rs"]
mod painter;

use painter::{Painter, diagonal_icon};
use vapors_text::AtlasConfig;

/// Holds four 16 px capitals, not eight (`gpu_atlas_upload.rs`).
const FIRST_PAGE: u32 = 32;
/// Holds every glyph drawn here.
const LARGEST_PAGE: u32 = 128;
const EIGHT_CAPITALS: &str = "ABCDEFGH";

fn growing() -> Painter {
    Painter::with_atlas(AtlasConfig {
        page_size: FIRST_PAGE,
        max_page_size: LARGEST_PAGE,
    })
}

#[test]
fn gpu_glyphs_that_overflow_the_first_page_draw_as_from_a_page_large_enough() {
    // Given: a frame already drawn from the first page's texture
    let mut painter = growing();
    painter.text("AB");
    let large = Painter::new(LARGEST_PAGE).text(EIGHT_CAPITALS);

    // When
    let drawn = painter.text(EIGHT_CAPITALS);

    // Then
    let side = painter.atlas.coverage_page().size();
    assert!(side > FIRST_PAGE, "the page did not grow: {side} px");
    assert!(
        drawn.pixels == large.pixels,
        "pixels differ from a large page"
    );
}

#[test]
fn gpu_an_icon_that_overflows_the_page_text_filled_this_frame_draws_as_from_a_page_large_enough() {
    // Given: four capitals fill the first page
    let icon = diagonal_icon();
    let large = Painter::new(LARGEST_PAGE).text_then_icon("ABCD", &icon);
    let mut painter = growing();

    // When
    let drawn = painter.text_then_icon("ABCD", &icon);

    // Then
    let side = painter.atlas.coverage_page().size();
    assert!(side > FIRST_PAGE, "the page did not grow: {side} px");
    assert!(
        drawn.pixels == large.pixels,
        "pixels differ from a large page"
    );
}
