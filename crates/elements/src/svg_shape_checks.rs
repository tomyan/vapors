//! What an svg on the shapes route refuses (`svg_route`), in two steps.
//! Before layout, each svg the update rechecks (it changed, or a path
//! of it did) is marked to be checked, or released when it left the
//! route or paints nothing (`display: none`, hidden). After layout, and
//! before paint, each marked svg, or one whose laid-out width changed,
//! has its shapes acquired at the device pixels per unit of its laid-out
//! content box, as paint will acquire them. Refused, naming the path: a
//! path the cache cannot make, and a shape reaching outside the view
//! box, since an svg drawn as shapes is not clipped yet (Chromium clips
//! it). So checks and paint read one width, and paint never refuses.

use std::cell::RefCell;

use vapors_dom::NodeId;

use crate::arena::Arena;
use crate::document::Document;
use crate::document_shapes::DocumentShapes;
use crate::error::ElementError;
use crate::layout::{BoxRect, Key, Placed};
use crate::slot_list::SlotList;
use crate::style::UsedMap;
use crate::svg_route::{Route, paints_nothing, route, why_shapes};
use crate::svg_shapes::{PaintedShape, SvgShapes, acquired, local_box, per_unit};

/// Slack for f32 rounding of a shape's box against the view box, as a
/// share of the view box's size.
const SLACK: f32 = 1e-4;

/// Marks `svg` to have its shapes checked after layout, or releases
/// them when it is off the shapes route or paints nothing: hidden, or
/// no path of it painting a fill or a stroke (`use_for(svg, &[])`, the
/// verify note on #379).
pub(crate) fn mark(
    (arena, used): (&Arena, &UsedMap),
    svg: NodeId,
    shapes: &RefCell<DocumentShapes>,
) {
    let mut shapes = shapes.borrow_mut();
    let paints = (crate::svg_route::shapes(arena, svg).iter())
        .any(|path| used.get(path).is_some_and(|own| !paints_nothing(own)));
    if route(arena, used, svg) == Some(Route::Shapes) && shown(used, svg) && paints {
        shapes.svgs.insert(svg, None);
    } else if shapes.svgs.contains_key(&svg) {
        shapes.release(svg);
    }
}

/// Whether `svg` paints by its own style: visible, and not `display:
/// none`. An ancestor's `display: none` leaves it no box, which the
/// check after layout releases (`laid_out_reasons`).
fn shown(used: &UsedMap, svg: NodeId) -> bool {
    used.get(&svg)
        .is_some_and(|own| own.visible && own.layout.display != taffy::Display::None)
}

impl Document {
    /// The refusals of each svg on the shapes route whose shapes need
    /// checking at its laid-out size: `placed`'s boxes at `rects`, at
    /// `scale` device pixels per CSS px. On a page with no svg on the
    /// shapes route this is one check of an empty map.
    pub(crate) fn svg_shape_refusals(
        &self,
        placed: &Placed,
        rects: &SlotList<BoxRect>,
        scale: f32,
    ) -> Result<(), Vec<ElementError>> {
        if self.shapes.borrow().svgs.is_empty() {
            return Ok(());
        }
        let mut svgs: Vec<NodeId> = self.shapes.borrow().svgs.keys().copied().collect();
        svgs.sort_unstable();
        let errors: Vec<ElementError> = svgs
            .into_iter()
            .flat_map(|svg| self.laid_out_reasons(svg, placed, rects, scale))
            .collect();
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// `svg`'s refusals at its laid-out content box; none when it was
    /// checked at this size, or has no box (released then).
    fn laid_out_reasons(
        &self,
        svg: NodeId,
        placed: &Placed,
        rects: &SlotList<BoxRect>,
        scale: f32,
    ) -> Vec<ElementError> {
        let slot = placed.slots.of(Key::Element(svg));
        let found = slot.and_then(|slot| Some((placed.boxes.get(slot)?, *rects.get(slot)?)));
        // no box: an ancestor has display: none, or the box was built
        // off the shapes route; either way it paints no shapes
        let Some((layout_box, border_box)) = found.filter(|(b, _)| b.svg_shapes.is_some()) else {
            self.shapes.borrow_mut().release(svg);
            return Vec::new();
        };
        let shapes = layout_box.svg_shapes.as_ref().expect("filtered");
        let content = crate::clip::content_box(layout_box, border_box);
        let at = per_unit(shapes.view_box, content, scale);
        if self.shapes.borrow().svgs.get(&svg) == Some(&Some(at)) {
            return Vec::new();
        }
        let reasons = self.reasons_at(svg, shapes, at);
        if reasons.is_empty() {
            self.shapes.borrow_mut().svgs.insert(svg, Some(at));
        }
        let element = crate::check::describe(&self.arena, svg);
        reasons
            .into_iter()
            .map(|reason| ElementError::UnsupportedAttribute(format!("{element}: {reason}")))
            .collect()
    }

    /// Why `svg`'s `shapes`, acquired at `at` device pixels per unit,
    /// cannot be drawn.
    fn reasons_at(&self, svg: NodeId, shapes: &SvgShapes, at: f32) -> Vec<String> {
        let used = &self.kept.as_ref().expect("styled keeps the styles").used;
        let parts: Vec<_> = (shapes.parts.iter())
            .map(|path| (path.site.as_str(), &path.data, &*used[&path.node]))
            .collect();
        let made = match acquired(&self.shapes, svg, &parts, at) {
            Ok(made) => made,
            Err(reason) => return vec![format!("a shape cannot be drawn: {reason}")],
        };
        let reach = outside(&self.shapes, &made, shapes.view_box);
        let overflow = |(part, reach): (usize, _)| {
            let site = shapes.parts[part].site.as_str();
            overflow(&self.arena, used, svg, (site, reach, shapes.view_box))
        };
        reach.map(overflow).into_iter().collect()
    }
}

/// The first shape of `made` reaching outside `view_box`: its path's
/// index and the box it draws.
fn outside(
    shapes: &RefCell<DocumentShapes>,
    made: &[PaintedShape],
    view_box: [f32; 4],
) -> Option<(usize, [[f32; 2]; 2])> {
    made.iter()
        .map(|shape| (shape.part, local_box(shapes, shape)))
        .find(|(_, reach)| !inside(*reach, view_box))
}

fn overflow(
    arena: &Arena,
    used: &UsedMap,
    svg: NodeId,
    (site, [lo, hi], [x, y, w, h]): (&str, [[f32; 2]; 2], [f32; 4]),
) -> String {
    let why = why_shapes(arena, used, svg).unwrap_or_default();
    format!(
        "its {site} draws from ({}, {}) to ({}, {}) in viewBox units, outside its viewBox \"{x} {y} {w} {h}\"; {why}, and an svg drawn as shapes is not clipped yet (the icon route clips to the box), so keep the shape and its stroke inside the viewBox",
        lo[0], lo[1], hi[0], hi[1]
    )
}

fn inside([lo, hi]: [[f32; 2]; 2], [x, y, w, h]: [f32; 4]) -> bool {
    let slack = SLACK * w.max(h);
    lo[0] >= x - slack && lo[1] >= y - slack && hi[0] <= x + w + slack && hi[1] <= y + h + slack
}
