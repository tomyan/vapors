use super::*;
use crate::atlas::AtlasConfig;
use crate::icon::IconStroke;
use crate::pipeline::FLOATS_PER_VERTEX;
use vapors_shapes::IconPath;

fn line_icon(y: &str) -> Icon {
    let path = IconPath::parse(&format!("M0 {y}H16")).expect("parses");
    let strokes = vec![IconStroke { path, width: 2.0 }];
    Icon::new([0.0, 0.0, 16.0, 16.0], strokes).expect("a valid icon")
}

fn draw(icon: &Icon, size: [u32; 2]) -> IconDraw<'_> {
    IconDraw {
        icon,
        origin: [0, 0],
        size,
        colour: Color::default(),
    }
}

/// The page pixel the quad's top-left vertex samples.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // whole page pixels
fn quad_top_left(floats: &[f32], quad: usize, page: u32) -> [u32; 2] {
    let vertex = &floats[quad * 6 * FLOATS_PER_VERTEX..];
    #[allow(clippy::cast_precision_loss)] // page sizes are far below 2^24
    let page = page as f32;
    [
        (vertex[2] * page).round() as u32,
        (vertex[3] * page).round() as u32,
    ]
}

#[test]
fn an_icon_moved_by_a_later_repack_in_the_same_call_samples_its_final_rect() {
    // Given: a tall icon from the last frame, then a small icon placed beside it
    // and a large one that only fits once the tall one is evicted and the page repacked
    let mut atlas = GlyphAtlas::new(AtlasConfig::fixed(40));
    let (tall, small, large) = (line_icon("2"), line_icon("4"), line_icon("6"));
    atlas.icon(&tall, 0, [8, 32]).expect("placed");
    atlas.begin_frame();

    // When
    let icons = [draw(&small, [8, 8]), draw(&large, [24, 24])];
    let floats = icon_quads(&mut atlas, &icons, None).expect("quads");

    // Then: the repack moved the small icon, and its quad samples where it is now
    assert_eq!(atlas.evictions(), 1);
    let small_rect = atlas.icon(&small, 0, [8, 8]).expect("cached").rect;
    assert_eq!(
        quad_top_left(&floats, 0, 40),
        [small_rect[0], small_rect[1]]
    );
}
