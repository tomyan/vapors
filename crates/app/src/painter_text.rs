//! The bytes of the painter's runs through the text pipeline and its
//! glyph atlas: text runs at their layout origins, and icons
//! (docs/icons.md, Paint) over their snapped content boxes, one quad per
//! path.

use vapors_elements::Painted;
use vapors_scene::{Item, Rect, Run};
use vapors_text::{AtlasError, AtlasPins, GlyphAtlas, IconDraw, TextDraw, TextPipeline};

/// The vertex bytes of text or icon `run` and the pins on what they
/// name (`painter_kept_text.rs`).
///
/// # Errors
///
/// [`AtlasError`] when its glyphs or icon masks cannot be placed.
pub(super) fn run_bytes(
    atlas: &mut GlyphAtlas,
    painted: &Painted,
    run: &Run,
) -> Result<(Vec<u8>, AtlasPins), AtlasError> {
    match run {
        Run::Texts(items) => TextPipeline::text_bytes(atlas, &text_draws(painted, items)),
        Run::Icons(items) => TextPipeline::icon_bytes(atlas, &icon_draws(painted, items)),
        other => unreachable!("{other:?} is not a text or icon run"),
    }
}

/// The bytes [`run_bytes`] would build now from where `atlas` holds the
/// glyphs, with no lookup; `None` when one is not held.
#[cfg(debug_assertions)]
pub(super) fn run_placed_bytes(
    atlas: &GlyphAtlas,
    painted: &Painted,
    run: &Run,
) -> Option<Vec<u8>> {
    match run {
        Run::Texts(items) => TextPipeline::placed_text_bytes(atlas, &text_draws(painted, items)),
        Run::Icons(items) => TextPipeline::placed_icon_bytes(atlas, &icon_draws(painted, items)),
        other => unreachable!("{other:?} is not a text or icon run"),
    }
}

/// The draws of the text items among `items`, in order.
fn text_draws<'a>(painted: &'a Painted, items: &[Item]) -> Vec<TextDraw<'a>> {
    let draw = |item: &Item| match *item {
        Item::Text { text, origin, .. } => Some(TextDraw {
            layout: painted.text(text),
            origin,
        }),
        _ => None,
    };
    items.iter().filter_map(draw).collect()
}

/// The draws of the icon items among `items`, in order.
fn icon_draws<'a>(painted: &'a Painted, items: &[Item]) -> Vec<IconDraw<'a>> {
    let draw = |item: &Item| match *item {
        Item::Icon { icon, rect, colour } => Some(icon_draw(painted.icon(icon), rect, colour)),
        _ => None,
    };
    items.iter().filter_map(draw).collect()
}

/// `icon` over `rect`, which paint snapped to whole device pixels.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // whole, non-negative pixels
fn icon_draw(icon: &vapors_text::Icon, rect: Rect, colour: vapors_scene::Color) -> IconDraw<'_> {
    IconDraw {
        icon,
        origin: [rect.left as i32, rect.top as i32],
        size: [rect.width().max(0.0) as u32, rect.height().max(0.0) as u32],
        colour,
    }
}
