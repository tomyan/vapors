//! What an svg on the shapes route refuses (`svg_route`): a path the
//! run-time cache cannot make (it fills the cache as it checks, so
//! paint finds every shape there), and a shape that reaches outside the
//! view box, since an svg's content is not clipped yet and Chromium
//! clips it (`overflow: hidden` on an inner svg).

use std::cell::RefCell;

use vapors_dom::NodeId;
use vapors_shapes::IconPath;

use crate::document_shapes::DocumentShapes;

use crate::arena::Arena;
use crate::style::UsedMap;
use crate::svg_route::{Route, path_site, paths, route, why_shapes};
use crate::svg_shapes::{acquired, local_box};

/// Slack for f32 rounding of a shape's box against the view box, as a
/// share of the view box's size.
const SLACK: f32 = 1e-4;

/// Why `svg`, an svg whose attributes the checks passed, is refused on
/// the shapes route, at `scale` device pixels per CSS px, naming the
/// path. Its shapes are acquired for it as they will be painted; an svg
/// that paints nothing (`display: none` or hidden) releases them.
pub(crate) fn reasons(
    (arena, used): (&Arena, &UsedMap),
    svg: NodeId,
    shapes: &RefCell<DocumentShapes>,
    scale: f32,
) -> Vec<String> {
    if route(arena, used, svg) != Some(Route::Shapes) {
        return Vec::new();
    }
    let (Some((view_box, per_unit)), Some(paths)) =
        (placed(arena, svg, scale), parsed(arena, used, svg))
    else {
        return Vec::new();
    };
    if !shown(arena, used, svg) {
        shapes.borrow_mut().release(svg);
        return Vec::new();
    }
    let to_paint: Vec<_> = (paths.iter())
        .map(|(site, data, own)| (site.as_str(), data, *own))
        .collect();
    let made = match acquired(shapes, svg, &to_paint, per_unit) {
        Ok(made) => made,
        Err(reason) => return vec![format!("a path cannot be drawn: {reason}")],
    };
    let reach = outside(shapes, &made, view_box);
    reach
        .map(|(path, reach)| overflow(arena, used, svg, (path, reach, view_box)))
        .into_iter()
        .collect()
}

/// Whether `svg` paints: it is visible, and neither it nor an ancestor
/// has `display: none`.
fn shown(arena: &Arena, used: &UsedMap, svg: NodeId) -> bool {
    let visible = used.get(&svg).is_some_and(|own| own.visible);
    let mut node = Some(svg);
    while let Some(at) = node {
        if used
            .get(&at)
            .is_some_and(|own| own.layout.display == taffy::Display::None)
        {
            return false;
        }
        node = arena.get(at, "svg_shape_checks").parent;
    }
    visible
}

/// The first shape of `made` reaching outside `view_box`: its path's
/// index and the box it draws.
fn outside(
    shapes: &RefCell<DocumentShapes>,
    made: &[crate::svg_shapes::PathShape],
    view_box: [f32; 4],
) -> Option<(usize, [[f32; 2]; 2])> {
    made.iter()
        .map(|shape| (shape.path, local_box(shapes, shape)))
        .find(|(_, reach)| !inside(*reach, view_box))
}

fn overflow(
    arena: &Arena,
    used: &UsedMap,
    svg: NodeId,
    (path, [lo, hi], [x, y, w, h]): (usize, [[f32; 2]; 2], [f32; 4]),
) -> String {
    let why = why_shapes(arena, used, svg).unwrap_or_default();
    format!(
        "its {} draws from ({}, {}) to ({}, {}) in viewBox units, outside its viewBox \"{x} {y} {w} {h}\"; {why}, and an svg drawn as shapes is not clipped yet (the icon route clips to the box), so keep the path and its stroke inside the viewBox",
        path_site(path),
        lo[0],
        lo[1],
        hi[0],
        hi[1]
    )
}

/// The view box and the device pixels per unit the svg draws at.
fn placed(arena: &Arena, svg: NodeId, scale: f32) -> Option<([f32; 4], f32)> {
    let element = arena.get(svg, "svg_shape_checks").element()?;
    let view_box = crate::svg::view_box(element.attributes.get("viewBox")?)?;
    let [width, _] = crate::svg::natural(arena, svg)?;
    Some((view_box, width / view_box[2] * scale))
}

/// Each path of `svg`: its description, its data and its used style.
fn parsed<'a>(
    arena: &Arena,
    used: &'a UsedMap,
    svg: NodeId,
) -> Option<Vec<(String, IconPath, &'a crate::style::Used)>> {
    paths(arena, svg)
        .into_iter()
        .enumerate()
        .map(|(index, path)| {
            let d = arena
                .get(path, "svg_shape_checks")
                .element()?
                .attributes
                .get("d")?;
            Some((
                path_site(index),
                IconPath::parse(d).ok()?,
                &**used.get(&path)?,
            ))
        })
        .collect()
}

fn inside([lo, hi]: [[f32; 2]; 2], [x, y, w, h]: [f32; 4]) -> bool {
    let slack = SLACK * w.max(h);
    lo[0] >= x - slack && lo[1] >= y - slack && hi[0] <= x + w + slack && hi[1] <= y + h + slack
}
