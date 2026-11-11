//! GPU tier: text and an icon recorded into one encoder each read their
//! own blend: the text's pixels match the text drawn alone, though the
//! icon's blend (gamma 1, no contrast boost) differs from the text's.

#[path = "support/painter.rs"]
mod painter;

use painter::{Painter, WIDTH, diagonal_icon};

/// The text is drawn from x = 2; the icon from x = 60.
const TEXT_COLUMNS: u32 = 56;
const PAGE: u32 = 128;

/// The RGBA bytes of the columns left of the icon, row by row.
fn text_columns(pixels: &[u8]) -> Vec<u8> {
    let (row, kept) = ((WIDTH * 4) as usize, (TEXT_COLUMNS * 4) as usize);
    pixels
        .chunks(row)
        .flat_map(|r| &r[..kept])
        .copied()
        .collect()
}

#[test]
fn gpu_text_then_an_icon_in_one_encoder_keeps_the_texts_blend() {
    // Given
    let alone = Painter::new(PAGE).text("Hello").pixels;
    let icon = diagonal_icon();

    // When
    let together = Painter::new(PAGE).text_then_icon("Hello", &icon).pixels;

    // Then
    assert!(
        text_columns(&alone).iter().any(|&byte| byte < 128),
        "the text left no ink to compare"
    );
    assert!(
        text_columns(&alone) == text_columns(&together),
        "the text drawn before an icon in one encoder differs from the text drawn alone"
    );
}
