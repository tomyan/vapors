use super::*;
use crate::atlas::{AtlasConfig, PageKind};
use crate::icon::IconStroke;
use crate::icon_path::IconPath;

fn line_icon(y: &str) -> Icon {
    let path = IconPath::parse(&format!("M0 {y}H16")).expect("parses");
    let strokes = vec![IconStroke { path, width: 2.0 }];
    Icon::new([0.0, 0.0, 16.0, 16.0], strokes).expect("a valid icon")
}

fn atlas(page_size: u32) -> GlyphAtlas {
    GlyphAtlas::new(AtlasConfig { page_size })
}

#[test]
fn an_icon_is_placed_in_the_coverage_page_at_its_device_size() {
    // Given
    let mut atlas = atlas(64);
    let icon = line_icon("8");

    // When
    let placed = atlas.icon(&icon, [16, 16]).expect("placed");

    // Then: its rect holds the icon's coverage, row by row
    assert_eq!(placed.page, PageKind::Coverage);
    assert_eq!([placed.rect[2], placed.rect[3]], [16, 16]);
    assert_eq!([placed.left, placed.top], [0, 0]);
    let expected = icon.rasterize([16, 16]);
    let page = atlas.coverage_page();
    for row in 0..16 {
        let start = ((placed.rect[1] + row) * page.size() + placed.rect[0]) as usize;
        let from = (row * 16) as usize;
        assert_eq!(
            &page.pixels()[start..start + 16],
            &expected.data[from..from + 16]
        );
    }
}

#[test]
fn one_icon_at_one_size_is_rasterised_once() {
    // Given
    let mut atlas = atlas(64);
    let icon = line_icon("8");

    // When
    let first = atlas.icon(&icon, [16, 16]).expect("placed");
    let again = atlas.icon(&line_icon("8"), [16, 16]).expect("placed");

    // Then
    assert_eq!(first, again);
    assert_eq!(atlas.rasterisations(), 1);
}

#[test]
fn another_size_or_another_path_is_another_entry() {
    // Given
    let mut atlas = atlas(64);
    atlas.icon(&line_icon("8"), [16, 16]).expect("placed");

    // When
    let larger = atlas.icon(&line_icon("8"), [32, 32]).expect("placed");
    let lower = atlas.icon(&line_icon("9"), [16, 16]).expect("placed");

    // Then
    assert_eq!(atlas.rasterisations(), 3);
    assert_eq!([larger.rect[2], larger.rect[3]], [32, 32]);
    assert_ne!(
        lower.rect,
        atlas.icon(&line_icon("8"), [16, 16]).unwrap().rect
    );
}

#[test]
fn an_icon_from_an_earlier_frame_is_evicted_to_make_room() {
    // Given: a page one icon and its padding fill
    let mut atlas = atlas(40);
    atlas.icon(&line_icon("8"), [32, 32]).expect("placed");
    atlas.begin_frame();

    // When
    let placed = atlas.icon(&line_icon("9"), [32, 32]).expect("placed");

    // Then
    assert_eq!(atlas.evictions(), 1);
    assert_eq!(placed.rect, [0, 0, 32, 32]);
}

#[test]
fn an_icon_larger_than_a_page_is_refused_naming_both_sizes() {
    // Given
    let mut atlas = atlas(32);

    // When
    let error = atlas.icon(&line_icon("8"), [48, 16]).expect_err("refused");

    // Then: nothing rasterised, and the message gives the fix
    assert_eq!(
        error,
        AtlasError::IconTooLarge {
            size: [48, 16],
            page_size: 32
        }
    );
    assert_eq!(atlas.rasterisations(), 0);
    assert!(error.to_string().contains("48x16"), "{error}");
    assert!(error.to_string().contains("Fix:"), "{error}");
}

#[test]
fn an_icon_fetched_again_this_frame_is_kept_and_the_page_reports_full() {
    // Given: an icon from the last frame, asked for again in this one
    let mut atlas = atlas(40);
    atlas.icon(&line_icon("8"), [32, 32]).expect("placed");
    atlas.begin_frame();
    atlas.icon(&line_icon("8"), [32, 32]).expect("cached");

    // When
    let error = atlas.icon(&line_icon("9"), [32, 32]).expect_err("no room");

    // Then
    assert!(matches!(error, AtlasError::Full { .. }), "{error:?}");
    assert_eq!(atlas.evictions(), 0);
}
