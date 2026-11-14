//! Test support, behind the `test-support` feature that only
//! dev-dependencies enable: scene items a test has an element paint
//! after its background, so a painter's run of a kind no element paints
//! yet goes through a real update, kept paint and kept batches. S3a1's
//! shape items use it until S4a's `<path>` paints them.

use std::cell::RefCell;
use std::collections::HashMap;

use vapors_dom::NodeId;
use vapors_scene::Item;

use crate::painting::Painting;

thread_local! {
    static ITEMS: RefCell<HashMap<NodeId, Vec<Item>>> = RefCell::new(HashMap::new());
}

/// Has `node` paint `items` after its background from its next paint
/// on, on this thread. A kept paint is not painted again for this, so
/// set them before the update that first paints `node`. The items are
/// keyed by node id alone and kept for the thread's life, so a second
/// document on the thread whose ids repeat paints them too.
pub fn paint_after_background(node: NodeId, items: Vec<Item>) {
    ITEMS.with(|map| map.borrow_mut().insert(node, items));
}

/// The items set for `node`, pushed into its paint.
pub(crate) fn paint(node: NodeId, out: &mut Painting) {
    ITEMS.with(|map| {
        for item in map.borrow().get(&node).into_iter().flatten() {
            out.scene.push(*item);
        }
    });
}

/// A shape item drawing `path` painted by `paint` from `doc`'s run-time
/// shape cache, as a `<path>` makes one (S4a): flattened for
/// `transform`'s scale, its kind from the cached geometry, in `colour`
/// within `bounds`. The painter's store takes the shape from the next
/// update's `Painted`.
///
/// # Panics
/// When the cache refuses the shape.
pub fn runtime_item(
    doc: &mut crate::Document,
    (path, paint): (&vapors_shapes::IconPath, vapors_shapes::Paint),
    transform: [f32; 6],
    colour: vapors_scene::Color,
    bounds: vapors_scene::Rect,
) -> Item {
    let [a, b, ..] = transform;
    let index = doc
        .shapes
        .entry("test_items", path, paint, a.hypot(b))
        .unwrap_or_else(|e| panic!("{e}"));
    let closed = doc.shapes.shapes()[index as usize]
        .geometry()
        .is_closed_form();
    let kind = if closed {
        vapors_scene::ShapeKind::ClosedForm
    } else {
        vapors_scene::ShapeKind::Mesh
    };
    let shape = vapors_scene::ShapeRef::runtime(index);
    Item::Shape(vapors_scene::Shape::new(
        shape, kind, transform, colour, bounds,
    ))
}
