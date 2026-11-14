//! Which route draws an inline `<svg>`'s paths (one-scene S4a): the
//! icon route (`svg_paint`, zeno into the glyph atlas) when every path
//! is in the icon subset, else shapes (`svg_shapes`), coverage on the
//! GPU after the transform. Moving icons onto shapes is the brief's
//! open question and not on this plan's path (one-scene 1.1, lead
//! 2026-11-14), so a path the icon route draws stays on it; a path it
//! cannot draw (a fill, a stroke in another colour) goes to shapes,
//! and with it the whole svg, so its paths keep their paint order.

use vapors_dom::NodeId;

use crate::arena::Arena;
use crate::style::{Used, UsedMap};

/// The route an svg's paths take.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Route {
    Icon,
    Shapes,
}

/// The route `svg`'s paths take; `None` before its paths are styled.
pub(crate) fn route(arena: &Arena, used: &UsedMap, svg: NodeId) -> Option<Route> {
    let own = used.get(&svg)?;
    let mut paths = paths(arena, svg).into_iter();
    let icon = paths.all(|path| used.get(&path).is_some_and(|path| icon_draws(path, own)));
    Some(if icon { Route::Icon } else { Route::Shapes })
}

/// Whether the icon route draws a path styled `own` in an svg styled
/// `svg`: no fill, a stroke in its own colour, which is its svg's, and
/// a width above zero.
pub(crate) fn icon_draws(own: &Used, svg: &Used) -> bool {
    own.svg.fill.is_none()
        && own.svg.stroke == Some(own.text.color)
        && own.svg.stroke_width > 0.0
        && own.text.color == svg.text.color
}

/// Whether a path styled `own` paints nothing on either route: no fill,
/// and no stroke or one of no width.
pub(crate) fn paints_nothing(own: &Used) -> bool {
    own.svg.fill.is_none() && (own.svg.stroke.is_none() || own.svg.stroke_width <= 0.0)
}

/// The `path` children of `svg`, in order.
pub(crate) fn paths(arena: &Arena, svg: NodeId) -> Vec<NodeId> {
    arena
        .children(svg)
        .into_iter()
        .filter(|&child| crate::svg::is(arena, child, "path"))
        .collect()
}
