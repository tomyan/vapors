//! Test support, behind the `test-support` feature that only
//! dev-dependencies enable: scene items a test has an element paint
//! after its background, so a painter's run of a kind no element paints
//! yet goes through a real update, kept paint and kept batches; and
//! run-time shapes an element paints from the document's cache, as a
//! `<path>` will (S4a), acquired in its paint and released when it
//! leaves the tree.

use std::cell::RefCell;
use std::collections::HashMap;

use vapors_dom::NodeId;
use vapors_scene::{Color, Item, Rect, Shape, ShapeRef};
use vapors_shapes::{IconPath, Paint};

use crate::paint::PaintInput;
use crate::painting::Painting;

/// A run-time shape an element paints: its path and paint, under the
/// transform (whose scale it is flattened for), in a colour, within
/// bounds.
#[derive(Clone, Debug)]
pub struct RuntimeItem {
    /// The path.
    pub path: IconPath,
    /// How it is painted.
    pub paint: Paint,
    /// From its units to device pixels; its scale picks the flattening.
    pub transform: [f32; 6],
    /// The paint's colour.
    pub colour: Color,
    /// What it draws, device pixels.
    pub bounds: Rect,
}

enum Painted {
    Items(Vec<Item>),
    Shapes(Vec<RuntimeItem>),
}

thread_local! {
    static PAINTS: RefCell<HashMap<NodeId, Painted>> = RefCell::new(HashMap::new());
}

/// Has `node` paint `items` after its background from its next paint
/// on, on this thread. A kept paint is not painted again for this, so
/// set them before the update that first paints `node`. The items are
/// keyed by node id alone and kept for the thread's life, so a second
/// document on the thread whose ids repeat paints them too.
pub fn paint_after_background(node: NodeId, items: Vec<Item>) {
    PAINTS.with(|map| map.borrow_mut().insert(node, Painted::Items(items)));
}

/// Has `node` paint `shapes` from its document's run-time cache after
/// its background, as [`paint_after_background`] does items.
pub fn paint_shapes_after_background(node: NodeId, shapes: Vec<RuntimeItem>) {
    PAINTS.with(|map| map.borrow_mut().insert(node, Painted::Shapes(shapes)));
}

/// What was set for `node`, pushed into its paint.
///
/// # Panics
/// When the cache refuses a shape.
pub(crate) fn paint(node: NodeId, input: &PaintInput<'_>, out: &mut Painting) {
    PAINTS.with(|map| match map.borrow().get(&node) {
        Some(Painted::Items(items)) => items.iter().for_each(|item| out.scene.push(*item)),
        Some(Painted::Shapes(shapes)) => paint_shapes(node, shapes, input, out),
        None => {}
    });
}

fn paint_shapes(node: NodeId, shapes: &[RuntimeItem], input: &PaintInput<'_>, out: &mut Painting) {
    let specs: Vec<_> = shapes
        .iter()
        .map(|s| {
            (
                "test_items",
                &s.path,
                s.paint,
                s.transform[0].hypot(s.transform[1]),
            )
        })
        .collect();
    let mut document = input.shapes.borrow_mut();
    let refs = document
        .use_for(node, &specs)
        .unwrap_or_else(|e| panic!("{e}"));
    for (shape, at) in shapes.iter().zip(refs) {
        let geometry = document.cache.get(vapors_shapes::CacheRef {
            index: at.index,
            generation: at.generation,
        });
        let closed = geometry.expect("just acquired").geometry().is_closed_form();
        let kind = if closed {
            vapors_scene::ShapeKind::ClosedForm
        } else {
            vapors_scene::ShapeKind::Mesh
        };
        let item = Shape::new(
            ShapeRef::Runtime(at),
            kind,
            shape.transform,
            shape.colour,
            shape.bounds,
        );
        out.scene.push(Item::Shape(item));
    }
}
