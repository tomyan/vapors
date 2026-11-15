//! An inline `<svg>` drawn as shapes (one-scene S4a; `svg_route` says
//! when): each path's fill, then its stroke, as SVG paints them, a
//! shape item each, acquired for the svg from the document's run-time
//! shapes (so they live while it paints them), under the transform from
//! the view box to the content box in device pixels.
//! The fill rule is nonzero, SVG's default; joins and caps are the
//! stroker's miter 4 and butt, as the icon route's.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::NodeId;
use vapors_scene::{Color, Item, Rect, RuntimeRef, Shape, ShapeKind, ShapeRef};
use vapors_shapes::{FillRule, IconPath, Paint, transformed_box};

use crate::document_shapes::DocumentShapes;

use crate::arena::Arena;
use crate::style::{Used, UsedMap};
use crate::svg_route::{Route, paths, route, shape_site};

/// An svg drawn as shapes: its view box and each path, its data parsed
/// once with its box, and the description that names it in messages.
#[derive(Debug, PartialEq)]
pub(crate) struct SvgShapes {
    pub(crate) view_box: [f32; 4],
    pub(crate) paths: Vec<SvgPath>,
}

/// One path of an svg drawn as shapes.
#[derive(Debug, PartialEq)]
pub(crate) struct SvgPath {
    pub(crate) node: NodeId,
    pub(crate) site: String,
    pub(crate) data: IconPath,
}

/// One shape a path paints: its entry, its stroke width (`None` for a
/// fill) and its colour.
pub(crate) struct PathShape {
    /// Which of the svg's paths paints it.
    pub(crate) path: usize,
    pub(crate) at: RuntimeRef,
    pub(crate) stroke: Option<f32>,
    pub(crate) colour: Color,
}

/// What `svg` draws as shapes, when it takes that route; `None` for any
/// other element, an svg on the icon route, or one the checks refuse.
pub(crate) fn of(arena: &Arena, used: &UsedMap, svg: NodeId) -> Option<Rc<SvgShapes>> {
    let element = arena.get(svg, "svg_shapes").element()?;
    if element.tag != "svg" || route(arena, used, svg)? != Route::Shapes {
        return None;
    }
    let view_box = crate::svg::view_box(element.attributes.get("viewBox")?)?;
    let paths = paths(arena, svg)
        .into_iter()
        .enumerate()
        .map(|(index, node)| {
            let data = parsed(arena, node)?;
            Some(SvgPath {
                node,
                site: shape_site(arena, node, index),
                data,
            })
        })
        .collect::<Option<_>>()?;
    Some(Rc::new(SvgShapes { view_box, paths }))
}

/// The path a shape draws: a path's `d`, or a basic shape's outline.
fn parsed(arena: &Arena, shape: NodeId) -> Option<IconPath> {
    let element = arena.get(shape, "svg_shapes").element()?;
    if element.tag == "path" {
        return IconPath::parse(element.attributes.get("d")?).ok();
    }
    let attribute = |name: &str| element.attributes.get(name).map(String::as_str);
    let outline = vapors_shapes::outline(&element.tag, attribute)?;
    IconPath::parse(&outline).ok()
}

/// The device pixels per view box unit an svg with `view_box` draws at
/// over its laid-out `content` box (CSS px) at `scale`: the view box
/// fitted inside the box (SVG's default `xMidYMid meet`), the one
/// answer its checks and its paint read.
pub(crate) fn per_unit(view_box: [f32; 4], content: crate::layout::BoxRect, scale: f32) -> f32 {
    (content.width / view_box[2]).min(content.height / view_box[3]) * scale
}

/// One path to paint: where it was written, its data, its used style.
pub(crate) type PathToPaint<'a> = (&'a str, &'a IconPath, &'a Used);

/// The shapes `svg` paints, each path's fill then its stroke, acquired
/// for `svg` from its document's shapes (releasing what it painted
/// before), drawn at `px_per_unit` device pixels per view box unit.
///
/// # Errors
/// What the cache refuses for a path, naming its site; `svg` keeps the
/// shapes it had.
pub(crate) fn acquired(
    shapes: &RefCell<DocumentShapes>,
    svg: NodeId,
    paths: &[PathToPaint<'_>],
    px_per_unit: f32,
) -> Result<Vec<PathShape>, String> {
    let painted = painted(paths);
    let specs: Vec<_> = (painted.iter())
        .map(|&(path, paint, _)| (paths[path].0, paths[path].1, paint, px_per_unit))
        .collect();
    let refs = shapes.borrow_mut().use_for(svg, &specs)?;
    Ok(refs
        .into_iter()
        .zip(painted)
        .map(|(at, (path, paint, colour))| PathShape {
            path,
            at,
            stroke: stroke_of(paint),
            colour,
        })
        .collect())
}

/// Each shape `paths` paint, in order: its path's index, paint and
/// colour.
fn painted(paths: &[PathToPaint<'_>]) -> Vec<(usize, Paint, Color)> {
    (paths.iter().enumerate())
        .flat_map(|(path, &(_, _, own))| {
            paints(own).map(move |(paint, colour)| (path, paint, colour))
        })
        .collect()
}

/// What a path styled `own` paints, in SVG's order: its fill, then its
/// stroke.
fn paints(own: &Used) -> impl Iterator<Item = (Paint, Color)> {
    let fill = own
        .svg
        .fill
        .map(|colour| (Paint::Fill(FillRule::NonZero), colour));
    let width = own.svg.stroke_width;
    let stroke =
        (own.svg.stroke.filter(|_| width > 0.0)).map(|colour| (Paint::Stroke { width }, colour));
    [fill, stroke].into_iter().flatten()
}

fn stroke_of(paint: Paint) -> Option<f32> {
    match paint {
        Paint::Stroke { width } => Some(width),
        Paint::Fill(_) => None,
    }
}

/// The box `shape` draws in view box units: a closed form's grown by
/// half its stroke, a mesh's holding its stroke.
pub(crate) fn local_box(shapes: &RefCell<DocumentShapes>, shape: &PathShape) -> [[f32; 2]; 2] {
    let shapes = shapes.borrow();
    let geometry = shapes.get(shape.at).expect("acquired, so live").geometry();
    let by = if geometry.is_closed_form() {
        shape.stroke.unwrap_or(0.0) / 2.0
    } else {
        0.0
    };
    geometry.local_box(by)
}

/// The kind of item `geometry` is drawn by.
pub(crate) fn kind_of(geometry: &vapors_shapes::Geometry<'_>) -> ShapeKind {
    if geometry.is_closed_form() {
        ShapeKind::ClosedForm
    } else {
        ShapeKind::Mesh
    }
}

/// The item drawing `shape` under `transform` (view box to device
/// pixels), its bounds what it draws there.
pub(crate) fn item(
    shapes: &RefCell<DocumentShapes>,
    shape: &PathShape,
    transform: [f32; 6],
) -> Item {
    let [lo, hi] = transformed_box(local_box(shapes, shape), transform);
    let kind = kind_of(
        &shapes
            .borrow()
            .get(shape.at)
            .expect("acquired, so live")
            .geometry(),
    );
    Item::Shape(Shape::new(
        ShapeRef::Runtime(shape.at),
        kind,
        transform,
        shape.colour,
        Rect::new(lo[0], lo[1], hi[0], hi[1]),
    ))
}
