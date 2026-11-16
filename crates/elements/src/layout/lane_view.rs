//! One lane of either kind as the lanes ask it (`lane_set.rs`): what
//! the block and flex lanes answer alike, so each question has one
//! dispatch.

use super::LayoutBox;
use super::lane::Lane;
use super::lane_block_answer::narrow;
use super::lane_kind::LaneKind;
use crate::lane_decline::LaneDecline;

/// A lane, of either kind.
pub(super) trait LaneView {
    /// Whether it holds the child in `slot`.
    fn holds(&self, slot: usize) -> bool;
    /// The child in `slot`'s start from the container's border box top.
    fn frame_offset(&self, slot: usize) -> Option<f64>;
    /// That start, when this lane wrote the child's layout, so the
    /// layout holds its relative offset alone; `None` before its first
    /// write, the layout holding Taffy's value then.
    fn written_start(&self, slot: usize) -> Option<f64>;
    /// The children whose box or overflow meets `from..=to`, each with
    /// its offset (`frame_offset`).
    fn children_between(&self, from: f64, to: f64) -> Vec<(usize, f64)>;
    /// The children from the one in `slot` on, in order.
    fn children_from(&self, slot: usize) -> Vec<usize>;
    /// Calls `each` with the children from the one in `slot` on, in
    /// order, each with its `frame_offset`: one fold to reach it, then
    /// one step a child, allocating nothing.
    fn each_offset_from(&self, slot: usize, each: &mut dyn FnMut(usize, f64));
    /// Whether its children can be frames (`Lane::framing`).
    fn framing(&self, origin: f64, scale: f32) -> Option<Result<(), LaneDecline>>;
    /// The child in `slot`'s `position: relative` offset along the axis.
    fn relative(&self, slot: usize) -> f32;
    /// The child in `slot` put in after `after`.
    fn child_inserted(&mut self, slot: usize, after: Option<usize>);
    /// The child in `slot` taken out, `next` in its place.
    fn child_removed(&mut self, slot: usize, next: Option<usize>);
    /// Puts each written child's start back in its layout in `boxes`,
    /// the lane of the container in `container` being dropped.
    fn restore_starts(&self, container: usize, boxes: &mut [LayoutBox]);
}

impl<K: LaneKind> LaneView for Lane<K> {
    fn holds(&self, slot: usize) -> bool {
        Lane::holds(self, slot)
    }

    fn frame_offset(&self, slot: usize) -> Option<f64> {
        Lane::frame_offset(self, slot)
    }

    fn written_start(&self, slot: usize) -> Option<f64> {
        if !self.has_written() {
            return None;
        }
        Lane::frame_offset(self, slot)
    }

    fn children_between(&self, from: f64, to: f64) -> Vec<(usize, f64)> {
        Lane::children_between(self, from, to)
    }

    fn children_from(&self, slot: usize) -> Vec<usize> {
        Lane::children_from(self, slot)
    }

    fn each_offset_from(&self, slot: usize, each: &mut dyn FnMut(usize, f64)) {
        let handle = *self.handles.get(&slot).unwrap_or_else(|| {
            panic!(
                "a started entry names slot {slot}, which its framed lane does not hold \
                 (layout/lane_write.rs `write_laid`); reproduce with `cargo xtask test`"
            )
        });
        let top = self.facts.top;
        self.tree.each_start_from(handle, &mut |index, start| {
            each(self.slots.at_index(index), top + start);
        });
    }

    fn framing(&self, origin: f64, scale: f32) -> Option<Result<(), LaneDecline>> {
        Lane::framing(self, origin, scale)
    }

    fn relative(&self, slot: usize) -> f32 {
        self.relative.get(&slot).copied().unwrap_or(0.0)
    }

    fn child_inserted(&mut self, slot: usize, after: Option<usize>) {
        Lane::child_inserted(self, slot, after);
    }

    fn child_removed(&mut self, slot: usize, next: Option<usize>) {
        Lane::child_removed(self, slot, next);
    }

    fn restore_starts(&self, container: usize, boxes: &mut [LayoutBox]) {
        if !self.has_written() {
            return;
        }
        let boxes_len = boxes.len();
        // a child laid out since the last write holds no written layout
        let written = self
            .handles
            .keys()
            .filter(|slot| !self.pending.contains_key(slot));
        for &slot in written {
            let Some(start) = Lane::frame_offset(self, slot) else {
                continue;
            };
            let layout_box = boxes.get_mut(slot).unwrap_or_else(|| {
                panic!(
                    "the lane of the container in slot {container} holds slot {slot}, past the \
                     {boxes_len} boxes: a child freed without telling its lane \
                     (`Lane::child_removed`); reproduce with `cargo test -p vapors-elements`"
                )
            });
            layout_box.layout.location.y += narrow(start);
        }
    }
}
