//! A range input's parts (3-E3a): a track across its content box,
//! centred vertically, and a round thumb at the value, both drawn from
//! the element's `--vapors-*` parts (`style::parts`). The track's
//! geometry is kept for the pointer, which maps a press to a value by
//! it (`range_input`).

use vapors_scene::{Item, Quad};

use crate::layout::{BoxRect, Content, LayoutBox};
use crate::paint::{PaintInput, snapped};
use crate::painted::Painted;

/// Where a range's thumb runs: its content box's left and width, and
/// the thumb's diameter, CSS px.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Track {
    pub(crate) x: f32,
    pub(crate) width: f32,
    pub(crate) thumb: f32,
}

/// Paints the track and thumb of `layout_box` when it is a range, over
/// its content box (border box `border_box`), and keeps its track.
pub(crate) fn paint(
    layout_box: &LayoutBox,
    border_box: BoxRect,
    input: &PaintInput<'_>,
    out: &mut Painted,
) {
    let (Some(range), &Content::Element(node)) = (&layout_box.range, &layout_box.content) else {
        return;
    };
    let used = &input.used[&node];
    let content = crate::clip::content_box(layout_box, border_box);
    let parts = used.parts;
    out.ranges.insert(
        node,
        Track {
            x: content.x,
            width: content.width,
            thumb: parts.thumb_size,
        },
    );
    if !used.visible {
        return;
    }
    let middle = content.y + content.height / 2.0;
    let track = BoxRect {
        x: content.x,
        y: middle - parts.track_height / 2.0,
        width: content.width,
        height: parts.track_height,
    };
    push_round(out, track, parts.track_color, input.scale);
    #[allow(clippy::cast_possible_truncation)] // a fraction in 0..=1
    let along = range.fraction() as f32 * (content.width - parts.thumb_size).max(0.0);
    let thumb = BoxRect {
        x: content.x + along,
        y: middle - parts.thumb_size / 2.0,
        width: parts.thumb_size,
        height: parts.thumb_size,
    };
    push_round(out, thumb, parts.thumb_color, input.scale);
}

/// A filled rect with fully rounded ends, snapped to device pixels.
fn push_round(out: &mut Painted, rect: BoxRect, color: vapors_scene::Color, scale: f32) {
    if rect.width <= 0.0 || rect.height <= 0.0 || color.a <= 0.0 {
        return;
    }
    let mut quad = Quad::fill(snapped(rect, scale), color);
    quad.radii = [rect.width.min(rect.height) * scale / 2.0; 4];
    out.scene.push(Item::Quad(quad));
}
