//! How much text one atlas page holds in one frame (review-fable-painter
//! H1): every glyph of an 80-character sample at each subpixel step, at
//! six sizes and two weights. A 1024 px page refuses it from 2x; the page
//! a 16384 px device gets (`AtlasConfig::for_max_texture_side`, 4096)
//! holds it at 3x.

use vapors_text::{
    AtlasConfig, AtlasError, Family, FontSystem, GlyphAtlas, GlyphRequest, Hinting, Layout,
    LayoutOptions, SUBPIXEL_STEPS, Style,
};

const SAMPLE: &str =
    "The quick brown fox jumps over the lazy dog, 0123456789 (AEIOU) $4.50 & 12% off!";
const SIZES: [f32; 6] = [12.0, 13.0, 14.0, 16.0, 20.0, 28.0];
const WEIGHTS: [f32; 2] = [400.0, 600.0];

/// The sample in every size and weight, laid out at `scale`.
fn layouts(system: &mut FontSystem, scale: f32) -> Vec<Layout> {
    let options = LayoutOptions {
        scale,
        ..LayoutOptions::default()
    };
    let styles = SIZES.iter().flat_map(|&size| {
        WEIGHTS.iter().map(move |&weight| Style {
            weight,
            ..Style::new(Family::Geist, size)
        })
    });
    styles
        .map(|style| Layout::new(system, SAMPLE, &style, &options))
        .collect()
}

/// Every glyph of `layouts` at every subpixel step, in one frame.
fn place_all(atlas: &mut GlyphAtlas, layouts: &[Layout], scale: f32) -> Result<(), AtlasError> {
    atlas.begin_frame();
    for glyph in layouts.iter().flat_map(Layout::glyphs) {
        for subpixel in 0..SUBPIXEL_STEPS {
            atlas.get(&GlyphRequest {
                font: glyph.font,
                id: glyph.id,
                size: glyph.size,
                coords: glyph.coords,
                subpixel,
                hinting: Hinting::for_scale(scale),
                skew: glyph.skew,
            })?;
        }
    }
    Ok(())
}

fn fits(page_size: u32, scale: f32) -> Result<(), AtlasError> {
    let mut system = FontSystem::bundle_only();
    let layouts = layouts(&mut system, scale);
    place_all(
        &mut GlyphAtlas::new(AtlasConfig { page_size }),
        &layouts,
        scale,
    )
}

#[test]
fn the_page_a_large_device_gets_holds_the_mixed_sample_at_every_scale_to_3x() {
    // Given
    let page_size = AtlasConfig::for_max_texture_side(16384).page_size;

    for scale in [1.0, 2.0, 2.625, 3.0] {
        // When
        let placed = fits(page_size, scale);

        // Then
        assert_eq!(placed, Ok(()), "{page_size} px page at {scale}x");
    }
}

#[test]
fn a_1024_page_refuses_the_mixed_sample_at_3x() {
    // When
    let placed = fits(1024, 3.0);

    // Then
    assert!(matches!(placed, Err(AtlasError::Full { .. })), "{placed:?}");
}
