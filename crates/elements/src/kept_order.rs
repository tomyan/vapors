//! Paint order, clip parents and scroll chains kept across updates and
//! walked again over the whole tree only when the tree's shape changed
//! (`changed_slots.rs`); clips kept by slot and redone under the
//! changed slots (`kept_clips.rs`). IU3 3c-3c, `docs/plans/iu3.md`
//! "What is kept, and what redoes it". Every box has an outline step,
//! so an outline added or removed changes no step. A debug build
//! compares each part with one made afresh (`kept_order_check.rs`).

use crate::changed_slots::ChangedSlots;
use crate::clip_tree::ClipTree;
use crate::kept_clips::Clips;
use crate::layout::{BoxRect, LayoutBox};
use crate::paint::PaintInput;
use crate::paint_order::{Step, paint_steps_with_outlines};
use crate::scroll_chains::Chains;

/// The steps of painting, where each box's clip comes from, each box's
/// clips, and the scroll chains.
#[derive(Debug, Default)]
pub(crate) struct KeptOrder {
    pub(crate) steps: Vec<Step>,
    pub(crate) tree: ClipTree,
    pub(crate) clips: Clips,
    pub(crate) chains: Chains,
    /// What the update that made this walked, for `UpdateCounts`.
    pub(crate) walks: OrderWalks,
}

/// Whole-tree walks of paint order and chains, and boxes whose clips
/// were redone.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct OrderWalks {
    pub(crate) order: usize,
    pub(crate) chains: usize,
    pub(crate) clips: usize,
}

impl KeptOrder {
    /// The order of `boxes` at `rects`, every part made afresh.
    pub(crate) fn of(boxes: &[LayoutBox], rects: &[BoxRect], input: &PaintInput<'_>) -> Self {
        let tree = ClipTree::of(boxes);
        let (clips, redone) = Clips::of(&tree, &|slot, own| {
            crate::clip::inner(boxes, rects, slot, own, input)
        });
        Self {
            steps: paint_steps_with_outlines(boxes, &|index| input.stackings.get(index)),
            tree,
            clips,
            chains: Chains::of(boxes, &|index| input.stackings.scrolls(index)),
            walks: OrderWalks {
                order: 1,
                chains: 1,
                clips: redone,
            },
        }
    }

    /// `last` kept for this update when the shape did not change, its
    /// clips redone under the `changed` slots; else made afresh.
    pub(crate) fn updated(
        last: Option<Self>,
        (boxes, rects): (&[LayoutBox], &[BoxRect]),
        input: &PaintInput<'_>,
        changed: &ChangedSlots,
    ) -> Self {
        match last {
            Some(kept) if !changed.shape_changed => kept.reclipped((boxes, rects), input, changed),
            _ => Self::of(boxes, rects, input),
        }
    }

    /// `self` with its clips redone under the `changed` slots.
    fn reclipped(
        mut self,
        (boxes, rects): (&[LayoutBox], &[BoxRect]),
        input: &PaintInput<'_>,
        changed: &ChangedSlots,
    ) -> Self {
        let slots = changed.rebuilt.iter().chain(&changed.moved).copied();
        let redone = self
            .clips
            .redo_under(slots.collect(), &self.tree, &|slot, own| {
                crate::clip::inner(boxes, rects, slot, own, input)
            });
        self.walks = OrderWalks {
            clips: redone,
            ..OrderWalks::default()
        };
        self
    }
}
