use vapors_text::{AtlasConfig, AtlasError, GlyphAtlas, Icon, IconPath, IconStroke};

use super::*;

/// A line icon that differs by `y`, so each is its own atlas entry.
fn line_icon(y: usize) -> Icon {
    let path = IconPath::parse(&format!("M0 {y}H16")).expect("parses");
    let strokes = vec![IconStroke { path, width: 2.0 }];
    Icon::new([0.0, 0.0, 16.0, 16.0], strokes).expect("a valid icon")
}

#[test]
fn nine_masks_of_the_largest_icon_size_fill_a_default_page() {
    // Given: a default atlas page and masks of MAX_DEVICE_PX, as docs/icons.md says
    let mut atlas = GlyphAtlas::new(AtlasConfig::default());
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let side = MAX_DEVICE_PX as u32;

    // When: masks are added in one frame until the page is full
    let fitted = (0..32)
        .take_while(|y| atlas.icon(&line_icon(*y), 0, [side, side]).is_ok())
        .count();

    // Then
    assert_eq!(fitted, 9);
    let error = atlas
        .icon(&line_icon(99), 0, [side, side])
        .expect_err("full");
    assert!(
        matches!(
            error,
            AtlasError::Full {
                icons_this_frame: 9,
                ..
            }
        ),
        "{error:?}"
    );
}
