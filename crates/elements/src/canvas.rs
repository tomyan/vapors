//! The canvas's colour, and the element it came from.

use taffy::Display;
use vapors_dom::NodeId;
use vapors_scene::Color;

use crate::paint::PaintInput;
use crate::painted::CanvasFrom;

/// The canvas's colour, as CSS Backgrounds 3 section 2.11.2 gives it:
/// the root's background, else the body's when the root's is
/// transparent, else white; and the element whose background it took,
/// which then paints none of its own. An element that is `display:
/// none`, or whose root is, gives none, as Chromium paints white then.
pub(crate) fn canvas_background(input: &PaintInput<'_>) -> (Color, Option<NodeId>) {
    let shown = |node| {
        input
            .used
            .get(&node)
            .filter(|used| used.layout.display != Display::None)
    };
    let Some(root) = shown(input.root) else {
        return (white(), None);
    };
    if root.background.a > 0.0 {
        return (root.background, Some(input.root));
    }
    match shown(input.body) {
        Some(body) if body.background.a > 0.0 => (body.background, Some(input.body)),
        _ => (white(), None),
    }
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

fn white() -> Color {
    Color::from_rgba8(255, 255, 255, 255)
}
