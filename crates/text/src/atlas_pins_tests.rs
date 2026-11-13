//! Pins keep the glyphs kept bytes name; a page that only pins hold
//! clears them, and the generation tells kept bytes to rebuild.

use super::*;
use crate::atlas::{Entry, glyph_at};
use crate::atlas_types::GlyphKey;
use crate::raster::{Coverage, Hinting};
use crate::{AtlasConfig, Family, FontSystem, Layout, LayoutOptions, Style, TextDraw};

fn key(id: u32) -> AtlasKey {
    AtlasKey::Glyph(GlyphKey {
        font: 1,
        index: 0,
        id,
        size_bits: 0,
        coords: Vec::new(),
        subpixel: 0,
        hinting: Hinting::None,
        skew_bits: 0,
    })
}

fn entry(last_used: u64) -> Entry {
    let coverage = Coverage {
        left: 0,
        top: 0,
        width: 4,
        height: 12,
        data: vec![255; 48],
        notdef: false,
    };
    Entry {
        glyph: glyph_at([0, 0], &coverage),
        coverage,
        last_used,
        frame: 1,
        pins: 0,
    }
}

/// A 13 px page holding two 4x12 glyphs from the last frame, the first
/// used least recently; a third fits only once one is evicted. The
/// first is pinned into the pins returned when `pin_first`, the second
/// when `pin_second`.
fn page_of_two(pin_first: bool, pin_second: bool) -> (GlyphAtlas, AtlasPins) {
    let mut atlas = GlyphAtlas::new(AtlasConfig::fixed(13));
    for id in [1, 2] {
        atlas.entries.insert(key(id), entry(u64::from(id)));
    }
    atlas.repack().expect("two fit");
    let mut pins = atlas.new_pins();
    for (id, pinned) in [(1, pin_first), (2, pin_second)] {
        atlas.pin(&key(id), pinned.then_some(&mut pins));
    }
    atlas.begin_frame();
    (atlas, pins)
}

fn pins_on(atlas: &GlyphAtlas, id: u32) -> Option<u32> {
    atlas.entries.get(&key(id)).map(|entry| entry.pins)
}

#[test]
fn a_pinned_glyph_is_kept_and_the_least_recent_unpinned_one_evicted() {
    // Given: the least recently used glyph is pinned
    let (mut atlas, _pins) = page_of_two(true, false);

    // When
    atlas.make_room(4, 12).expect("room");

    // Then
    assert_eq!(pins_on(&atlas, 1), Some(1));
    assert_eq!(pins_on(&atlas, 2), None);
    assert_eq!(atlas.evictions(), 1);
}

#[test]
fn a_page_only_pins_hold_clears_every_pin_evicts_and_moves_the_generation_on() {
    // Given
    let (mut atlas, pins) = page_of_two(true, true);
    let generation = atlas.generation();

    // When
    atlas.make_room(4, 12).expect("room");

    // Then: the least recent is evicted, the other unpinned
    assert_eq!(pins_on(&atlas, 1), None);
    assert_eq!(pins_on(&atlas, 2), Some(0));
    assert!(atlas.generation() > generation);
    atlas.unpin(pins); // taken before the clear: void, not an underflow
    assert_eq!(pins_on(&atlas, 2), Some(0));
}

#[test]
fn an_unpinned_glyph_is_evicted_again() {
    // Given
    let (mut atlas, pins) = page_of_two(true, false);
    atlas.unpin(pins);

    // When
    atlas.make_room(4, 12).expect("room");

    // Then
    assert_eq!(pins_on(&atlas, 1), None);
    assert_eq!(pins_on(&atlas, 2), Some(0));
}

#[test]
#[should_panic(expected = "pinned 1 time")]
fn evicting_a_pinned_glyph_panics_naming_it_and_its_pins() {
    // Given
    let (mut atlas, _pins) = page_of_two(true, false);

    // When
    atlas.evict(&key(1));
}

#[test]
fn every_repack_moves_the_generation_on() {
    // Given
    let (mut atlas, _pins) = page_of_two(false, false);
    let generation = atlas.generation();

    // When
    atlas.repack().expect("fits");

    // Then
    assert_eq!(atlas.generation(), generation + 1);
}

#[test]
fn each_glyph_of_a_text_is_pinned_once_on_the_second_pass_and_lookups_are_counted() {
    // Given
    let mut system = FontSystem::bundle_only();
    let style = Style::new(Family::Geist, 16.0);
    let layout = Layout::new(&mut system, "Ab", &style, &LayoutOptions::default());
    let mut atlas = GlyphAtlas::new(AtlasConfig::fixed(256));
    let mut pins = atlas.new_pins();
    let text = TextDraw {
        layout: &layout,
        origin: [0.0, 20.0],
    };

    // When
    crate::quads::quads(&mut atlas, &[text], Some(&mut pins)).expect("quads");

    // Then: two glyphs, each fetched on both passes
    assert_eq!(pins.len(), 2);
    assert_eq!(atlas.lookups(), 4);
    assert!(atlas.entries.values().all(|entry| entry.pins == 1));
}

#[test]
fn pins_made_before_a_clear_and_taken_after_it_are_given_back() {
    // Given: pins made, then every pin cleared before any is taken, as
    // when a build's first pass clears them
    let (mut atlas, held) = page_of_two(true, false);
    let mut pins = atlas.new_pins();
    assert!(atlas.unpin_all());

    // When
    atlas.pin(&key(2), Some(&mut pins));
    atlas.unpin(pins);

    // Then
    assert_eq!(pins_on(&atlas, 2), Some(0));
    atlas.unpin(held); // void: taken before the clear
}
