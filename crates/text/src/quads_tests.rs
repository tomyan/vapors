use super::*;
use crate::atlas::PageKind;
use crate::pipeline::FLOATS_PER_VERTEX;
use crate::{AtlasConfig, Family, FontSystem, LayoutOptions, Style};

const VIEWPORT: Size = Size {
    width: 64,
    height: 64,
};

fn layout(system: &mut FontSystem, text: &str, size: f32) -> Layout {
    let style = Style::new(Family::Geist, size);
    Layout::new(system, text, &style, &LayoutOptions::default())
}

/// `layouts` drawn in one run at (0, 40), each its own text.
fn run_quads(atlas: &mut GlyphAtlas, layouts: &[Layout]) -> Vec<f32> {
    let texts: Vec<TextDraw<'_>> = layouts
        .iter()
        .map(|layout| TextDraw {
            layout,
            origin: [0.0, 40.0],
        })
        .collect();
    quads(atlas, &texts, VIEWPORT).expect("quads")
}

/// A 30 px page holding a tall l (7x29) from the last frame, so that a
/// period (3x3) fits beside it and an M (18x17) only once the l is
/// evicted and the page repacked.
fn page_holding_a_tall_l(system: &mut FontSystem) -> GlyphAtlas {
    let mut atlas = GlyphAtlas::new(AtlasConfig::fixed(30));
    run_quads(&mut atlas, &[layout(system, "l", 40.0)]);
    atlas.begin_frame();
    atlas
}

#[test]
fn a_glyph_moved_by_a_later_repack_in_the_same_layout_samples_its_final_rect() {
    // Given
    let mut system = FontSystem::bundle_only();
    let mut atlas = page_holding_a_tall_l(&mut system);

    // When
    let floats = run_quads(&mut atlas, &[layout(&mut system, ".M", 24.0)]);

    // Then: the repack moved the period to the corner, and its quad samples it there
    assert_eq!(atlas.evictions(), 1);
    assert_close(&floats[2..4], &[0.0, 0.0]);
}

#[test]
fn a_glyph_moved_by_a_repack_for_a_later_text_in_the_run_samples_its_final_rect() {
    // Given
    let mut system = FontSystem::bundle_only();
    let mut atlas = page_holding_a_tall_l(&mut system);
    let texts = [
        layout(&mut system, ".", 24.0),
        layout(&mut system, ".M", 24.0),
    ];

    // When
    let floats = run_quads(&mut atlas, &texts);

    // Then: the first text's period sits where the second text's repack moved it
    assert_eq!(atlas.evictions(), 1);
    assert_close(&floats[2..4], &[0.0, 0.0]);
}

#[test]
fn a_quad_maps_the_glyph_rect_to_its_atlas_coordinates() {
    // Given: a 4x2 glyph at page (8, 16) in a 32 px page, placed at (10, 20)
    let glyph = AtlasGlyph {
        page: PageKind::Coverage,
        rect: [8, 16, 4, 2],
        left: 0,
        top: 0,
        notdef: false,
    };
    let viewport = Size {
        width: 100,
        height: 50,
    };
    let mut floats = Vec::new();

    // When
    push_quad(
        &mut floats,
        [10.0, 20.0],
        &glyph,
        32,
        viewport,
        Color::default(),
    );

    // Then: six vertices; the first is the top-left, the fifth the bottom-right
    assert_eq!(floats.len(), 6 * FLOATS_PER_VERTEX);
    assert_close(&floats[0..4], &[-0.8, 0.2, 0.25, 0.5]);
    assert_close(
        &floats[4 * FLOATS_PER_VERTEX..][..4],
        &[-0.72, 0.12, 0.375, 0.5625],
    );
}

fn assert_close(actual: &[f32], expected: &[f32]) {
    assert_eq!(actual.len(), expected.len());
    for (a, e) in actual.iter().zip(expected) {
        assert!((a - e).abs() < 1e-6, "{actual:?} vs {expected:?}");
    }
}
