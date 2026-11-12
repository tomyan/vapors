use super::*;
use crate::{AtlasConfig, Family, FontSystem, Layout, LayoutOptions, Style};

fn hello(system: &mut FontSystem) -> Layout {
    let style = Style::new(Family::Geist, 16.0);
    Layout::new(system, "Hello", &style, &LayoutOptions::default())
}

fn draw(layout: &Layout) -> TextDraw<'_> {
    TextDraw {
        layout,
        origin: [2.0, 20.0],
    }
}

#[test]
fn placed_bytes_are_the_built_bytes_and_look_nothing_up() {
    // Given
    let mut system = FontSystem::bundle_only();
    let layout = hello(&mut system);
    let mut atlas = GlyphAtlas::new(AtlasConfig::fixed(256));
    let (built, _pins) = TextPipeline::text_bytes(&mut atlas, &[draw(&layout)]).expect("built");
    let lookups = atlas.lookups();

    // When
    let placed = TextPipeline::placed_text_bytes(&atlas, &[draw(&layout)]);

    // Then
    assert_eq!(placed, Some(built));
    assert_eq!(atlas.lookups(), lookups);
}

#[test]
fn placed_bytes_are_none_once_a_glyph_is_gone() {
    // Given
    let mut system = FontSystem::bundle_only();
    let layout = hello(&mut system);
    let mut atlas = GlyphAtlas::new(AtlasConfig::fixed(256));
    let (_, pins) = TextPipeline::text_bytes(&mut atlas, &[draw(&layout)]).expect("built");
    atlas.unpin(pins);
    let gone = atlas.entries.keys().next().expect("a glyph").clone();
    atlas.evict(&gone);

    // When
    let placed = TextPipeline::placed_text_bytes(&atlas, &[draw(&layout)]);

    // Then
    assert_eq!(placed, None);
}
