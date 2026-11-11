//! The canvas's colour, and the element it came from.

use vapors_dom::NodeId;
use vapors_scene::Color;

use crate::layout::{Content, LayoutBox};
use crate::paint::PaintInput;
use crate::painted::CanvasFrom;

/// The canvas's colour, as CSS Backgrounds 3 section 2.11.2 gives it:
/// the root's background, else the body's when the root's is
/// transparent, else white; and the element whose background it took,
/// which then paints none of its own. An element with no box (`display:
/// none`, or under one) gives none, as Chromium paints white then.
pub(crate) fn canvas_background(
    boxes: &[LayoutBox],
    input: &PaintInput<'_>,
) -> (Color, Option<NodeId>) {
    for node in [input.root, input.body] {
        match input.used.get(&node) {
            Some(used) if used.background.a > 0.0 && boxed(boxes, node) => {
                return (used.background, Some(node));
            }
            _ => {}
        }
    }
    (Color::from_rgba8(255, 255, 255, 255), None)
}

/// Which of html and body the `propagated` background came from.
pub(crate) fn canvas_from(propagated: Option<NodeId>, root: NodeId) -> Option<CanvasFrom> {
    propagated.map(|node| {
        if node == root {
            CanvasFrom::Root
        } else {
            CanvasFrom::Body
        }
    })
}

fn boxed(boxes: &[LayoutBox], node: NodeId) -> bool {
    boxes.iter().any(
        |layout_box| matches!(layout_box.content, Content::Element(element) if element == node),
    )
}
