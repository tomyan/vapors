//! The `rects` and `paddings` maps of a paint (each element's border
//! box and padding box's top-left), kept from the last paint and
//! patched only for the slots this update changed (IU3 3c-3b,
//! `docs/plans/iu3.md`): a rebuilt or moved element's entries replaced,
//! a freed one's removed. The rects of a text run's inline elements are
//! its part's entries, put and removed with it (`kept_box.rs`). Each
//! element's entries are in its scroll frame (`frames.rs`), noted in
//! `node_frames` beside them. A debug
//! build compares the maps with maps built afresh
//! (`kept_paint_check.rs`).

use crate::changed_slots::ChangedSlots;
use crate::kept_order::KeptOrder;
use crate::layout::{BoxRect, Content, Key, LayoutBox};
use crate::painted::Painted;

/// Fills `painted`'s maps: its `kept` maps patched for `changed`, or
/// built from every box when nothing is kept. Notes how many elements'
/// entries were replaced or removed.
pub(crate) fn fill(painted: &mut Painted, placed: Placed<'_>, kept: bool, changed: &ChangedSlots) {
    painted.maps_patched = if kept {
        patch(painted, placed, changed)
    } else {
        build(painted, placed)
    };
}

/// The boxes, their rects and the kept order, which holds each one's
/// scroll frame.
pub(crate) type Placed<'a> = (&'a [LayoutBox], &'a [BoxRect], &'a KeptOrder);

/// Puts every element's entries into `painted`'s maps; how many.
pub(crate) fn build(painted: &mut Painted, placed: Placed<'_>) -> usize {
    let put = |slot: &usize| put(painted, placed, *slot);
    (0..placed.0.len()).filter(put).count()
}

/// Removes the freed elements' entries, then puts the rebuilt and moved
/// elements' entries; how many elements. Removing first lets a node
/// freed and built again in one update keep its new entries. An
/// element's frame changes only with a box built again: a box that
/// starts or stops scrolling, or changes containing block, is restyled,
/// which builds its subtree again.
fn patch(painted: &mut Painted, placed: Placed<'_>, changed: &ChangedSlots) -> usize {
    let removed = changed
        .freed
        .iter()
        .filter(|(_, key)| remove(painted, *key))
        .count();
    let slots = changed.rebuilt.iter().chain(&changed.moved);
    removed + slots.filter(|slot| put(painted, placed, **slot)).count()
}

/// Removes an element's entries; whether `key` is an element's.
fn remove(painted: &mut Painted, key: Key) -> bool {
    let Key::Element(node) = key else {
        return false;
    };
    painted.rects.remove(&node);
    painted.paddings.remove(&node);
    painted.node_frames.remove(&node);
    true
}

/// Puts the entries of the element at `slot`; whether it holds one.
fn put(painted: &mut Painted, (boxes, rects, order): Placed<'_>, slot: usize) -> bool {
    let Content::Element(node) = boxes[slot].content else {
        return false;
    };
    painted.rects.insert(node, rects[slot]);
    painted.node_frames.insert(node, order.clips.frame(slot));
    let padding = crate::clip::padding_box(&boxes[slot], rects[slot]);
    painted.paddings.insert(node, [padding.x, padding.y]);
    true
}
