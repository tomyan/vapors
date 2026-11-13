//! The surface items of a run: each one's name, backing size and rect.

use vapors_elements::Painted;
use vapors_scene::{Item, Rect};

use crate::surface::SurfaceSize;

/// Each surface item of `items`: its name, backing size and rect.
pub(super) fn surfaces<'a>(
    painted: &'a Painted,
    items: &'a [Item],
) -> impl Iterator<Item = (&'a str, SurfaceSize, Rect)> + 'a {
    items.iter().filter_map(|item| match item {
        Item::Surface { surface, rect } => Some((
            painted.surface(*surface),
            backing_size(*rect, painted.scale()),
            *rect,
        )),
        _ => None,
    })
}

/// The device-pixel size of a surface's snapped `rect` at `scale`.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // a painted rect is non-empty
fn backing_size(rect: Rect, scale: f32) -> SurfaceSize {
    SurfaceSize {
        width: rect.width().round() as u32,
        height: rect.height().round() as u32,
        scale,
    }
}
