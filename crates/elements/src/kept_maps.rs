//! The `rects` and `paddings` maps of a paint (each element's border
//! box and padding box's top-left), kept from the last paint and
//! patched only for the slots this update changed (IU3 3c-3b,
//! `docs/plans/iu3.md`): a rebuilt or moved element's entries replaced,
//! a freed one's removed. A debug build compares them with maps built
//! afresh (`kept_paint_check.rs`).

use crate::changed_slots::ChangedSlots;
use crate::layout::{BoxRect, Content, Key, LayoutBox};
use crate::node_map::NodeMap;
use crate::painted::Painted;

/// The maps a paint keeps for the next: `rects`, then `paddings`.
pub(crate) type ElementMaps = (NodeMap<BoxRect>, NodeMap<[f32; 2]>);

/// Fills `painted`'s maps: `last`'s patched for `changed`, or built
/// from every box when nothing is kept. Notes how many elements' entries
/// were replaced or removed.
pub(crate) fn fill(
    painted: &mut Painted,
    (boxes, rects): (&[LayoutBox], &[BoxRect]),
    last: Option<ElementMaps>,
    changed: &ChangedSlots,
) {
    painted.maps_patched = match last {
        Some(maps) if !changed.every => {
            (painted.rects, painted.paddings) = maps;
            patch(painted, (boxes, rects), changed)
        }
        _ => build(painted, (boxes, rects)),
    };
}

/// Puts every element's entries into `painted`'s maps; how many.
pub(crate) fn build(painted: &mut Painted, (boxes, rects): (&[LayoutBox], &[BoxRect])) -> usize {
    let put = |slot: &usize| put(painted, boxes, rects, *slot);
    (0..boxes.len()).filter(put).count()
}

/// Removes the freed elements' entries, then puts the rebuilt and moved
/// ones'; how many elements. Removing first lets a node freed and built
/// again in one update keep its new entries.
fn patch(
    painted: &mut Painted,
    (boxes, rects): (&[LayoutBox], &[BoxRect]),
    changed: &ChangedSlots,
) -> usize {
    let removed = changed
        .freed
        .iter()
        .filter(|(_, key)| remove(painted, *key))
        .count();
    let slots = changed.rebuilt.iter().chain(&changed.moved);
    removed
        + slots
            .filter(|slot| put(painted, boxes, rects, **slot))
            .count()
}

/// Removes an element's entries; whether `key` is an element's.
fn remove(painted: &mut Painted, key: Key) -> bool {
    let Key::Element(node) = key else {
        return false;
    };
    painted.rects.remove(&node);
    painted.paddings.remove(&node);
    true
}

/// Puts the entries of the element at `slot`; whether it holds one.
fn put(painted: &mut Painted, boxes: &[LayoutBox], rects: &[BoxRect], slot: usize) -> bool {
    let Content::Element(node) = boxes[slot].content else {
        return false;
    };
    painted.rects.insert(node, rects[slot]);
    let padding = crate::clip::padding_box(&boxes[slot], rects[slot]);
    painted.paddings.insert(node, [padding.x, padding.y]);
    true
}
