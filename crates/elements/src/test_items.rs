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
/// set them before the update that first paints `node`.
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
