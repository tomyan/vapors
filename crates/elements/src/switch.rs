//! A switch: `input type=checkbox switch`, a checkbox drawn as a
//! rounded track and a round knob. The track is the content box (the
//! user-agent sheet sizes it 2.25em by 1.25em), in `accent-color` when
//! checked (else the user agent's blue) and in `--vapors-track-color`
//! when not; the knob is white, inset by an eleventh of the height
//! (2 px of a 22 px track) and at the end when checked. Behaviour is the
//! checkbox's (`checkbox.rs`); the role is `switch` (`a11y_roles.rs`).
//! With `appearance: none` it draws neither, so the box's own
//! background and radius can be the track and a sibling the knob.

use vapors_scene::Color;

use crate::layout::{BoxRect, Content, LayoutBox};
use crate::node::Element;
use crate::paint::PaintInput;
use crate::painting::Painting;
use crate::range_paint::push_round;

/// The knob's inset from the track's edges, as a fraction of its height.
const INSET: f32 = 1.0 / 11.0;

/// Whether `element` is a checkbox with the `switch` attribute.
pub(crate) fn is_switch(element: &Element) -> bool {
    element.attributes.contains_key("switch")
        && element.tag == "input"
        && crate::check::is_type(element, "checkbox")
}

impl crate::document::Document {
    /// Whether `node` is a switch.
    pub(crate) fn is_switch(&self, node: vapors_dom::NodeId) -> bool {
        self.arena
            .get(node, "switch")
            .element()
            .is_some_and(is_switch)
    }
}

/// Paints the track and knob of `layout_box` when it is a switch, over
/// its content box (border box `border_box`).
pub(crate) fn paint(
    layout_box: &LayoutBox,
    border_box: BoxRect,
    input: &PaintInput<'_>,
    out: &mut Painting,
) {
    let (true, Some(checked), &Content::Element(node)) =
        (layout_box.switch, layout_box.checked, &layout_box.content)
    else {
        return;
    };
    let used = &input.used[&node];
    if !used.visible || !used.native() {
        return; // `appearance: none` draws neither (`style/appearance.rs`)
    }
    let track = crate::clip::content_box(layout_box, border_box);
    let fill = if checked {
        used.accent
            .unwrap_or(crate::style::parts::DEFAULT.mark_color)
    } else {
        used.parts.track_color
    };
    push_round(out, track, fill, input);
    push_round(
        out,
        knob(track, checked),
        Color::from_rgba8(255, 255, 255, 255),
        input,
    );
}

/// The knob: a circle as tall as the track less its insets, at the
/// start or, when `checked`, the end.
fn knob(track: BoxRect, checked: bool) -> BoxRect {
    let inset = track.height * INSET;
    let size = (track.height - 2.0 * inset).max(0.0);
    let x = if checked {
        track.x + track.width - inset - size
    } else {
        track.x + inset
    };
    BoxRect {
        x,
        y: track.y + inset,
        width: size,
        height: size,
    }
}
