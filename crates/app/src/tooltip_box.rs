//! Where the tooltip's box and text go: the anchor its trigger maps to,
//! the width its text wraps at, and the box around the measured text,
//! placed by [`place_tooltip`] and snapped to device pixels.

use vapors_elements::Painted;
use vapors_scene::{Quad, Rect};

use crate::tooltip_error::TooltipError;
use crate::tooltip_style::TooltipStyle;
use crate::{Anchor, Point, Tooltip, TooltipTrigger, place_tooltip};

/// The widest a tooltip's text runs before it wraps, CSS px: views'
/// `kTooltipMaxWidth` (`ui/views/corewm/tooltip_aura.cc`). It limits the
/// text, not the box: `TooltipViewAura::ResetDisplayRect` gives the
/// text this width and `CalculatePreferredSize` adds the insets outside
/// it (`tooltip_view_aura.cc`, read 2026-11-10), as the padding and
/// border are added here.
pub(crate) const MAX_TEXT_WIDTH: f32 = 800.0;

/// What the painter draws, in device pixels: the box, and the text's
/// top-left.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TooltipBox {
    pub quad: Quad,
    pub text_origin: [f32; 2],
}

/// The width the text wraps at in a window `window_width` CSS px wide:
/// `TooltipAura::GetMaxWidth`, `min(kTooltipMaxWidth, (width + 1) / 2)`
/// of the display, here of the window, which is what vapors knows.
pub(crate) fn wrap_width(window_width: f32) -> f32 {
    MAX_TEXT_WIDTH.min(f32::midpoint(window_width, 1.0).floor())
}

/// The anchor `tooltip`'s trigger maps to: the pointer's hotspot, or
/// the border box of the focused element.
pub(crate) fn anchor_of(
    tooltip: &Tooltip,
    pointer: Option<[f32; 2]>,
    painted: &Painted,
) -> Result<Anchor, TooltipError> {
    match tooltip.trigger {
        TooltipTrigger::Pointer => {
            let [x, y] = pointer.ok_or(TooltipError::NoPointer)?;
            Ok(Anchor::Pointer(Point { x, y }))
        }
        TooltipTrigger::Keyboard => painted
            .border_box(tooltip.node)
            .map(Anchor::Keyboard)
            .ok_or(TooltipError::NoBox(tooltip.node)),
    }
}

/// The box for text `text_size` CSS px in `style`, placed at `anchor`
/// in a `window` of CSS px, at `scale` device pixels per CSS pixel.
pub(crate) fn tooltip_box(
    style: &TooltipStyle,
    text_size: [f32; 2],
    anchor: Anchor,
    window: [f32; 2],
    scale: f32,
) -> Result<TooltipBox, TooltipError> {
    let size = outer_size(style, text_size);
    let at = place_tooltip(anchor, size, window).map_err(TooltipError::Placement)?;
    Ok(TooltipBox {
        quad: quad(style, at, size, scale),
        text_origin: text_origin(style, at, scale),
    })
}

/// The text's top-left in device pixels: inside the border as it is
/// drawn (whole device pixels), then the padding, rounded as Blink
/// snaps a content box.
fn text_origin(style: &TooltipStyle, at: Point, scale: f32) -> [f32; 2] {
    let border = border_pixels(style.border_width, scale);
    let [top, _, _, left] = style.padding;
    [(at.x, left), (at.y, top)].map(|(edge, pad)| (edge * scale + border + pad * scale).round())
}

/// The border box around text of `text_size`.
fn outer_size(style: &TooltipStyle, text_size: [f32; 2]) -> [f32; 2] {
    let [top, right, bottom, left] = style.padding;
    let border = 2.0 * style.border_width;
    [
        text_size[0] + left + right + border,
        text_size[1] + top + bottom + border,
    ]
}

/// The box as a quad, each edge on the nearest device pixel, as the
/// document's boxes are.
fn quad(style: &TooltipStyle, at: Point, size: [f32; 2], scale: f32) -> Quad {
    let snap = |v: f32| (v * scale).round();
    let rect = Rect::new(
        snap(at.x),
        snap(at.y),
        snap(at.x + size[0]),
        snap(at.y + size[1]),
    );
    let border = border_pixels(style.border_width, scale);
    Quad {
        border: [border; 4],
        border_colors: [style.border_color; 4],
        radii: [style.border_radius * scale; 4],
        ..Quad::fill(rect, style.background)
    }
}

/// A border width in whole device pixels, as Blink snaps the
/// document's (vapors-elements' `snap_width`): floored, and at least 1
/// when it is not 0.
fn border_pixels(width: f32, scale: f32) -> f32 {
    if width == 0.0 {
        return 0.0;
    }
    (width * scale).floor().max(1.0)
}

#[cfg(test)]
#[path = "tooltip_box_tests.rs"]
mod tests;
