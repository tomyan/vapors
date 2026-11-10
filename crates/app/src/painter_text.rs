//! The painter's runs through the text pipeline and its glyph atlas:
//! text runs at their layout origins, and icons (docs/icons.md, Paint)
//! over their snapped content boxes, one quad per path.

use vapors_elements::Painted;
use vapors_renderer::Target;
use vapors_scene::{Item, Rect};
use vapors_text::{AtlasError, BlendParams, IconDraw, RenderTarget};

use super::ScenePainter;

impl ScenePainter {
    pub(super) fn draw_texts(
        &mut self,
        target: &Target<'_>,
        painted: &Painted,
        items: &[Item],
    ) -> Result<(), AtlasError> {
        let text_target = text_target(target);
        for item in items {
            if let Item::Text { text, origin } = item {
                let layout = painted.text(*text);
                let blend = BlendParams::default();
                let atlas = &mut self.atlas;
                self.text
                    .draw(&text_target, atlas, layout, *origin, blend)?;
            }
        }
        Ok(())
    }

    /// Draws a run's icons in one call, in document order.
    pub(super) fn draw_icons(
        &mut self,
        target: &Target<'_>,
        painted: &Painted,
        items: &[Item],
    ) -> Result<(), AtlasError> {
        let draws: Vec<IconDraw<'_>> = items
            .iter()
            .filter_map(|item| match *item {
                Item::Icon { icon, rect, colour } => {
                    Some(icon_draw(painted.icon(icon), rect, colour))
                }
                _ => None,
            })
            .collect();
        self.text
            .draw_icons(&text_target(target), &mut self.atlas, &draws)
    }
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

fn text_target<'a>(target: &Target<'a>) -> RenderTarget<'a> {
    RenderTarget {
        device: target.device,
        queue: target.queue,
        view: target.view,
        viewport: target.viewport,
        scissor: target.scissor,
    }
}
