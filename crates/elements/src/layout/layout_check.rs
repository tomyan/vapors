//! Debug builds check every update that built by owners against a
//! layout of a fresh box tree with no cache (review-v217 finding 2). A
//! box kept or lent a cache that is out of date, such as a shallow
//! owner given back its old cache after a box below it changed, lays
//! out the same boxes elsewhere or at another size; the walk panics at
//! the first such box, naming its slot and key, at the update where it
//! happens. `rebuild_check.rs` compares the boxes, not their layout.
//!
//! Debug timings (`cargo xtask dev`, touche's debug page) include this
//! full build and layout, O(tree), on every narrowed update. What would
//! retire it: a randomized oracle test that makes several structural
//! changes per update, nested shallow owners in either slot order
//! among them, so the class is caught in tests without the per-update
//! cost.

use taffy::Layout;
use vapors_dom::NodeId;
use vapors_text::FontSystem;

use super::boxes::{Source, build};
use super::{Calcs, Placed, Slots, TextCache, lay_out};
use crate::update::Viewport;

/// What laying out reads besides the boxes.
pub(crate) struct Inputs<'a> {
    pub(crate) fonts: &'a mut FontSystem,
    pub(crate) calcs: &'a Calcs,
    pub(crate) viewport: Viewport,
}

/// Lays out a fresh build of `root`'s boxes, with a fresh text cache,
/// and walks it from the root beside `placed`; panics at the first box
/// whose size or location differ.
pub(crate) fn check_layout_against_fresh(
    placed: &Placed,
    source: &Source<'_>,
    root: NodeId,
    layout: Inputs<'_>,
) {
    walk(&fresh_layout(source, root, layout), placed);
}

/// `root`'s boxes built and laid out with no cache.
fn fresh_layout(source: &Source<'_>, root: NodeId, layout: Inputs<'_>) -> Placed {
    let fresh = Placed {
        boxes: build(source, root),
        slots: Slots::default(),
    };
    let viewport = layout.viewport;
    let texts = (layout.fonts, &mut TextCache::default());
    let size = (viewport.width, viewport.height);
    let laid = lay_out(fresh, texts, viewport.scale, size, layout.calcs, Vec::new());
    laid.unwrap_or_else(|(failure, _)| panic!("a fresh layout failed: {failure}"))
        .0
}

fn walk(fresh: &Placed, placed: &Placed) {
    let mut stack = vec![(0, 0)];
    while let Some((index, slot)) = stack.pop() {
        let (new, kept) = (&fresh.boxes[index].layout, &placed.boxes[slot].layout);
        if new.size != kept.size || new.location != kept.location {
            stale(placed, slot, (new, kept));
        }
        let new_children = fresh.boxes[index].children.iter().copied();
        stack.extend(new_children.zip(placed.boxes[slot].children.iter().copied()));
    }
}

fn stale(placed: &Placed, slot: usize, (new, kept): (&Layout, &Layout)) -> ! {
    panic!(
        "the box in slot {slot}, {:?}, is laid out at {:?} {:?} where a fresh layout puts it \
         at {:?} {:?}: a kept or lent cache is out of date (docs/plans/splice.md (b)); \
         `cargo test -p vapors-elements` reproduces it",
        placed.slots.key(slot),
        kept.location,
        kept.size,
        new.location,
        new.size,
    )
}
