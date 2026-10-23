//! CPU rasterisation of single glyphs from the bundled fonts.

use std::sync::Arc;

use parley::FontData;
use parley::fontique::Blob;
use skrifa::MetadataProvider;

use super::*;
use crate::fonts::GEIST;

fn geist() -> FontData {
    FontData::new(Blob::new(Arc::new(GEIST)), 0)
}

fn glyph_id(font: &FontData, c: char) -> u32 {
    let font_ref = skrifa::FontRef::new(font.data.as_ref()).expect("font");
    font_ref.charmap().map(c).expect("mapped").to_u32()
}

fn request(font: &FontData, id: u32, subpixel: u8, hinting: Hinting) -> GlyphRequest<'_> {
    GlyphRequest {
        font,
        id,
        size: 16.0,
        coords: &[],
        subpixel,
        hinting,
        skew: 0.0,
    }
}

fn centroid_x(coverage: &Coverage) -> f32 {
    let (mut weighted, mut total) = (0.0f32, 0.0f32);
    for y in 0..coverage.height {
        for x in 0..coverage.width {
            let value = f32::from(coverage.data[(y * coverage.width + x) as usize]);
            #[allow(clippy::cast_precision_loss)] // small bitmaps
            let fx = x as f32;
            weighted += fx * value;
            total += value;
        }
    }
    weighted / total
}

#[test]
fn a_capital_i_at_16px_is_a_vertical_stem_above_the_baseline() {
    // Given
    let font = geist();
    let mut rasterizer = Rasterizer::new();

    // When
    let coverage = rasterizer
        .rasterize(&request(&font, glyph_id(&font, 'I'), 0, Hinting::None))
        .expect("rasterises");

    // Then: Geist's cap height is about 0.71 em, so 11 or 12 rows at 16 px
    assert!((11..=12).contains(&coverage.height), "{coverage:?}");
    assert!((1..=4).contains(&coverage.width), "{coverage:?}");
    assert_eq!(
        coverage.top,
        -(coverage.height.cast_signed()),
        "ink ends on the baseline"
    );
    let row = (coverage.height / 2 * coverage.width) as usize;
    let middle_row = &coverage.data[row..row + coverage.width as usize];
    let stem = middle_row
        .iter()
        .map(|&v| f32::from(v) / 255.0)
        .sum::<f32>();
    assert!(
        (1.0..=2.5).contains(&stem),
        "stem {stem} px from {middle_row:?}"
    );
    assert!(!coverage.notdef);
}

#[test]
fn a_subpixel_offset_moves_the_ink_right_by_a_quarter_pixel_per_step() {
    // Given
    let font = geist();
    let mut rasterizer = Rasterizer::new();
    let id = glyph_id(&font, 'l');

    // When
    let at_zero = rasterizer
        .rasterize(&request(&font, id, 0, Hinting::None))
        .expect("ok");
    let at_two = rasterizer
        .rasterize(&request(&font, id, 2, Hinting::None))
        .expect("ok");

    // Then
    assert_ne!(at_zero.data, at_two.data);
    #[allow(clippy::cast_precision_loss)] // small offsets
    let shift =
        (centroid_x(&at_two) + at_two.left as f32) - (centroid_x(&at_zero) + at_zero.left as f32);
    assert!((shift - 0.5).abs() < 0.1, "shift {shift}");
}

#[test]
fn light_hinting_changes_the_bitmap_at_one_x_and_is_off_at_two_x() {
    // Given
    let font = geist();
    let mut rasterizer = Rasterizer::new();
    let id = glyph_id(&font, 'e');

    // When
    let unhinted = rasterizer
        .rasterize(&request(&font, id, 0, Hinting::None))
        .expect("ok");
    let hinted = rasterizer
        .rasterize(&request(&font, id, 0, Hinting::Light))
        .expect("ok");

    // Then
    assert_ne!(unhinted.data, hinted.data);
    assert_eq!(Hinting::for_scale(1.0), Hinting::Light);
    assert_eq!(Hinting::for_scale(1.5), Hinting::Light);
    assert_eq!(Hinting::for_scale(2.0), Hinting::None);
}

#[test]
fn a_space_has_no_ink() {
    // Given
    let font = geist();
    let mut rasterizer = Rasterizer::new();

    // When
    let coverage = rasterizer
        .rasterize(&request(&font, glyph_id(&font, ' '), 0, Hinting::Light))
        .expect("ok");

    // Then
    assert!(coverage.is_empty());
}

#[test]
fn a_glyph_the_font_does_not_have_draws_notdef_and_says_so() {
    // Given
    let font = geist();
    let mut rasterizer = Rasterizer::new();

    // When
    let coverage = rasterizer
        .rasterize(&request(&font, 999_999, 0, Hinting::Light))
        .expect("falls back");

    // Then
    assert!(coverage.notdef);
    assert!(!coverage.is_empty(), ".notdef is visible");
}

#[test]
fn bytes_that_are_not_a_font_are_refused_with_the_fix() {
    // Given
    let font = FontData::new(Blob::new(Arc::new(vec![0u8; 16])), 0);
    let mut rasterizer = Rasterizer::new();

    // When
    let error = rasterizer
        .rasterize(&request(&font, 0, 0, Hinting::Light))
        .expect_err("not a font");

    // Then
    let message = error.to_string();
    assert!(message.contains("not a font"), "{message}");
    assert!(message.contains("Fix:"), "{message}");
}

#[test]
fn x_positions_split_into_a_pixel_and_a_quarter_step() {
    // Given
    let cases = [
        (10.0, (10, 0)),
        (10.13, (10, 1)),
        (10.5, (10, 2)),
        (10.9, (11, 0)),
        (-0.3, (-1, 3)),
    ];

    for (x, expected) in cases {
        // When
        let split = subpixel_of(x);

        // Then
        assert_eq!(split, expected, "x = {x}");
    }
}
