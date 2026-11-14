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
use crate::svg_route::{Route, paths, route};
use crate::svg_shapes::{acquired, local_box};

/// Slack for f32 rounding of a shape's box against the view box, as a
/// share of the view box's size.
const SLACK: f32 = 1e-4;

/// Why `svg`, an svg whose attributes the checks passed, is refused on
/// the shapes route, at `scale` device pixels per CSS px, naming the
/// path. Its shapes are acquired for it as they will be painted.
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
    let to_paint: Vec<_> = paths
        .iter()
        .map(|(site, data, own)| (site.as_str(), data, *own))
        .collect();
    let made = match acquired(shapes, svg, &to_paint, per_unit) {
        Ok(made) => made,
        Err(reason) => return vec![format!("a <path> cannot be drawn: {reason}")],
    };
    made.iter()
        .filter_map(|shape| {
            let reach = local_box(shapes, shape);
            (!inside(reach, view_box)).then(|| format!(
                "its <path> {} draws over {reach:?} in viewBox units, outside the viewBox {view_box:?}; an svg's content is not clipped yet, so keep the path and its stroke inside the viewBox",
                paths[shape.path].0
            ))
        })
        .collect()
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
        .map(|path| {
            let d = arena
                .get(path, "svg_shape_checks")
                .element()?
                .attributes
                .get("d")?;
            Some((
                crate::check::describe(arena, path),
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
