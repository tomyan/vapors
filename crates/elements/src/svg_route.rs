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

/// The route `svg`'s paths take; `None` before the svg is styled.
///
/// # Panics
/// When the svg is styled and a path of it is not: styling gives every
/// element a used style, as paint relies on.
pub(crate) fn route(arena: &Arena, used: &UsedMap, svg: NodeId) -> Option<Route> {
    let own = used.get(&svg)?;
    let mut paths = paths(arena, svg).into_iter();
    let icon = paths.all(|path| {
        let path = used.get(&path).unwrap_or_else(|| {
            panic!("a path of a styled svg has no used style; styling gives every element one")
        });
        icon_draws(path, own)
    });
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

/// Why `svg` is on the shapes route: the first path the icon route
/// cannot draw, by its index, and what it has; `None` on the icon route.
pub(crate) fn why_shapes(arena: &Arena, used: &UsedMap, svg: NodeId) -> Option<String> {
    let own = used.get(&svg)?;
    let (index, path) = paths(arena, svg)
        .into_iter()
        .enumerate()
        .find(|(_, path)| !used.get(path).is_some_and(|path| icon_draws(path, own)))?;
    let has = used
        .get(&path)
        .map_or("no used style", |path| what_it_has(path, own));
    Some(format!(
        "it is drawn as shapes because {} {has}",
        path_site(index)
    ))
}

/// What a path styled `own` in an svg styled `svg` has that keeps it
/// off the icon route.
fn what_it_has(own: &Used, svg: &Used) -> &'static str {
    if own.svg.fill.is_some() {
        "has a fill"
    } else if own.text.color != svg.text.color {
        "has a colour of its own"
    } else if own.svg.stroke != Some(own.text.color) {
        "has a stroke other than its colour"
    } else {
        "has a stroke of no width"
    }
}

/// How messages name an svg's path: by its index among the svg's paths,
/// counting from 1, which a bare path's description cannot tell apart.
pub(crate) fn path_site(index: usize) -> String {
    format!("path {}", index + 1)
}

/// The `path` children of `svg`, in order.
pub(crate) fn paths(arena: &Arena, svg: NodeId) -> Vec<NodeId> {
    arena
        .children(svg)
        .into_iter()
        .filter(|&child| crate::svg::is(arena, child, "path"))
        .collect()
}
