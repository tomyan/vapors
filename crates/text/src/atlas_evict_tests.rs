//! Repacking does not depend on map order: the reviewer's shape, four
//! 4x4 and two 4x1 glyphs in a 13 px page, fits tallest first (12 px of
//! shelves) and not interleaved (15 px).

use super::*;
use crate::atlas::{AtlasConfig, Entry};
use crate::raster::{Coverage, Hinting};

fn key(id: u32) -> GlyphKey {
    GlyphKey {
        font: 1,
        index: 0,
        id,
        size_bits: 0,
        coords: Vec::new(),
        subpixel: 0,
        hinting: Hinting::None,
    }
}

fn entry(width: u32, height: u32, last_used: u64) -> Entry {
    let coverage = Coverage {
        left: 0,
        top: 0,
        width,
        height,
        data: vec![255; (width * height) as usize],
        notdef: false,
    };
    Entry {
        glyph: glyph_at([0, 0], &coverage),
        coverage,
        last_used,
        frame: 1,
    }
}

#[test]
fn repack_fits_the_reviewers_shape_whatever_order_the_map_holds_it_in() {
    // Given: six glyphs in a 13 px page, inserted interleaved
    let mut atlas = GlyphAtlas::new(AtlasConfig { page_size: 13 });
    for (id, (w, h)) in [(4, 1), (4, 4), (4, 1), (4, 4), (4, 4), (4, 4)]
        .into_iter()
        .enumerate()
    {
        let id = u32::try_from(id).expect("six");
        atlas.entries.insert(key(id), entry(w, h, u64::from(id)));
    }

    // When
    atlas.repack().expect("fits tallest first");

    // Then: every glyph has a distinct place inside the page
    let mut origins: Vec<[u32; 2]> = atlas
        .entries
        .values()
        .map(|e| [e.glyph.rect[0], e.glyph.rect[1]])
        .collect();
    origins.sort_unstable();
    origins.dedup();
    assert_eq!(origins.len(), 6, "{origins:?}");
    assert_eq!(atlas.evictions(), 0);
    assert!(
        atlas
            .entries
            .values()
            .all(|e| e.glyph.rect[1] + e.glyph.rect[3] <= 13)
    );
}
