//! Paint order, clip parents and scroll chains kept across updates,
//! spliced when the tree's shape changed by in-flow boxes inserted or
//! removed (`order_splice.rs`) and walked again over the whole tree
//! when it changed otherwise (`changed_slots.rs`); clips kept by slot
//! and redone under the changed slots (`kept_clips.rs`). IU3 3c-3c,
//! `docs/plans/iu3.md` "What is kept, and what redoes it". Every box
//! has an outline step, so an outline added or removed changes no
//! step. A debug build compares each part with one made afresh
//! (`kept_order_check.rs`).

use crate::changed_slots::ChangedSlots;
use crate::clip_tree::ClipTree;
use crate::kept_clips::Clips;
use crate::layout::{BoxRect, LayoutBox};
use crate::order_splice::Place;
use crate::paint::PaintInput;
use crate::paint_order::{Step, paint_steps_with_outlines};
use crate::scroll_chains::Chains;
use crate::splice_decline::SpliceDecline;

/// The steps of painting, where each box's clip comes from, each box's
/// clips, and the scroll chains.
#[derive(Debug, Default)]
pub(crate) struct KeptOrder {
    pub(crate) steps: Vec<Step>,
    pub(crate) tree: ClipTree,
    pub(crate) clips: Clips,
    pub(crate) chains: Chains,
    /// The scroll containers, whose thumbs are painted on every update
    /// that paints.
    pub(crate) thumbs: Vec<usize>,
    /// The slots whose clips the update that made this redid.
    pub(crate) redone: Vec<usize>,
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
    /// Shape changes spliced into the kept order (`order_splice.rs`).
    pub(crate) splices: usize,
    /// Why the splice declined a shape change, walked instead.
    pub(crate) declined: Option<SpliceDecline>,
}

impl KeptOrder {
    /// The order of `boxes` at `rects`, every part made afresh.
    pub(crate) fn of(boxes: &[LayoutBox], rects: &[BoxRect], input: &PaintInput<'_>) -> Self {
        let tree = ClipTree::of(boxes);
        let (clips, redone) = Clips::of(&tree, &|slot, own| {
            crate::clip::inner(boxes, rects, slot, own, input)
        });
        let steps = paint_steps_with_outlines(boxes, &|index| input.stackings.get(index));
        Self {
            thumbs: thumbs(&steps),
            steps,
            tree,
            clips,
            chains: Chains::of(boxes, &|index| input.stackings.scrolls(index)),
            walks: OrderWalks {
                order: 1,
                chains: 1,
                clips: redone.len(),
                ..OrderWalks::default()
            },
            redone,
        }
    }

    /// `last` kept for this update when the shape did not change, its
    /// clips redone under the `changed` slots; spliced when it changed
    /// by in-flow boxes inserted or removed, `place` giving each kept
    /// step's place; else made afresh.
    pub(crate) fn updated(
        last: Option<Self>,
        (boxes, rects): (&[LayoutBox], &[BoxRect]),
        input: &PaintInput<'_>,
        changed: &ChangedSlots,
        place: Place<'_>,
    ) -> Self {
        match last {
            Some(kept) if !changed.shape_changed => kept.reclipped((boxes, rects), input, changed),
            Some(kept) if !changed.every => {
                kept.spliced_or_walked((boxes, rects), input, changed, place)
            }
            _ => Self::of(boxes, rects, input),
        }
    }

    /// `self` spliced for the `changed` slots when the change can be
    /// (`order_splice.rs`), else made afresh with the decline counted.
    fn spliced_or_walked(
        self,
        (boxes, rects): (&[LayoutBox], &[BoxRect]),
        input: &PaintInput<'_>,
        changed: &ChangedSlots,
        place: Place<'_>,
    ) -> Self {
        let splice = match crate::order_splice::plan(boxes, input.stackings, changed, place) {
            Ok(splice) => splice,
            Err(reason) => {
                let mut walked = Self::of(boxes, rects, input);
                walked.walks.declined = Some(reason);
                return walked;
            }
        };
        self.spliced(splice, (boxes, rects), input, changed)
    }

    /// `self` with its clips redone under the `changed` slots.
    pub(crate) fn reclipped(
        mut self,
        (boxes, rects): (&[LayoutBox], &[BoxRect]),
        input: &PaintInput<'_>,
        changed: &ChangedSlots,
    ) -> Self {
        let slots = changed.rebuilt.iter().chain(&changed.moved).copied();
        self.redone = self
            .clips
            .redo_under(slots.collect(), &self.tree, &|slot, own| {
                crate::clip::inner(boxes, rects, slot, own, input)
            });
        self.walks = OrderWalks {
            clips: self.redone.len(),
            ..OrderWalks::default()
        };
        self
    }
}

/// The slots of `steps`' thumb steps.
fn thumbs(steps: &[Step]) -> Vec<usize> {
    let thumb = |step: &Step| matches!(step, Step::Thumb(_)).then(|| step.index());
    steps.iter().filter_map(thumb).collect()
}
