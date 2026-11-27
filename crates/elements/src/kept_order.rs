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
use crate::kept_clips::{ClipInputs, Clips};
use crate::layout::{BoxRect, LayoutBox};
use crate::paint::PaintInput;
use crate::paint_labels::Label;
use crate::paint_order::{Step, paint_steps_with_outlines};
use crate::scroll_chains::Chains;
pub(crate) use labels::LabelOf;
pub(crate) use walks::OrderWalks;

/// The steps of painting, where each box's clip comes from, each box's
/// clips, and the scroll chains.
#[derive(Debug, Default)]
pub(crate) struct KeptOrder {
    pub(crate) steps: Vec<Step>,
    /// Each step's label, increasing along `steps` (`paint_labels.rs`):
    /// what every reader of paint order compares.
    pub(crate) labels: Vec<Label>,
    /// The spacing of the last walk's labels, which an append or a
    /// prepend steps by.
    pub(crate) spacing: u64,
    /// The first and last labels of each run the last splice put in,
    /// which a run of the next leans on when it touches one
    /// (`paint_labels::Lean`).
    pub(crate) recent: Vec<(Label, Label)>,
    /// What the splice that made this did, for the drawing steps, the
    /// extents index and the hits to follow; empty unless it was spliced.
    pub(crate) splice_edits: crate::kept_order_splice::SpliceEdits,
    pub(crate) tree: ClipTree,
    pub(crate) clips: Clips,
    pub(crate) chains: Chains,
    /// The scroll containers, each with a thumb step.
    pub(crate) thumbs: Vec<usize>,
    /// The slots whose clips the update that made this redid.
    pub(crate) redone: Vec<usize>,
    /// What the update that made this walked, for `UpdateCounts`.
    pub(crate) walks: OrderWalks,
}

impl KeptOrder {
    /// The order of `boxes` at `rects`, every part made afresh.
    pub(crate) fn of(boxes: &[LayoutBox], rects: &[BoxRect], input: &PaintInput<'_>) -> Self {
        let tree = ClipTree::of(boxes);
        let (clips, redone) = Clips::of(
            &tree,
            &BoxClips {
                boxes,
                rects,
                input,
            },
        );
        let steps = paint_steps_with_outlines(boxes, &|index| input.stackings.get(index));
        Self {
            thumbs: thumbs(&steps),
            labels: crate::paint_labels::afresh(steps.len()),
            spacing: crate::paint_labels::spacing(steps.len()),
            recent: Vec::new(),
            splice_edits: crate::kept_order_splice::SpliceEdits::default(),
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
    /// by in-flow boxes inserted or removed, `label_of` giving each kept
    /// step's label; else made afresh.
    pub(crate) fn updated(
        last: Option<Self>,
        (boxes, rects): (&[LayoutBox], &[BoxRect]),
        input: &PaintInput<'_>,
        changed: &ChangedSlots,
        label_of: LabelOf<'_>,
    ) -> Self {
        match last {
            Some(kept) if !changed.shape_changed => kept.reclipped((boxes, rects), input, changed),
            Some(kept) if !changed.every => {
                kept.spliced_or_walked((boxes, rects), input, changed, label_of)
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
        label_of: LabelOf<'_>,
    ) -> Self {
        let place = |step| label_of(step).map(|label| self.position_of(label));
        let splice = match crate::order_splice::plan(boxes, input.stackings, changed, &place) {
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
        self.redone = self.clips.redo_under(
            slots.collect(),
            &self.tree,
            &BoxClips {
                boxes,
                rects,
                input,
            },
        );
        self.walks = OrderWalks {
            clips: self.redone.len(),
            ..OrderWalks::default()
        };
        // what a splice did is the splice's to set, after this
        self.splice_edits = crate::kept_order_splice::SpliceEdits::default();
        self
    }
}

/// What clips read of `boxes` at `rects`: the clip each gives its
/// content, and whether it scrolls.
struct BoxClips<'a, 'b> {
    boxes: &'a [LayoutBox],
    rects: &'a [BoxRect],
    input: &'a PaintInput<'b>,
}

impl ClipInputs for BoxClips<'_, '_> {
    fn inner(
        &self,
        slot: usize,
        own: Option<crate::clip::BoxClip>,
    ) -> crate::kept_clips::InnerClip {
        let inner = crate::clip::inner(self.boxes, self.rects, slot, own, self.input);
        let framing = self.input.placement.framing.get(&slot);
        let frames = self.scrolls(slot) || matches!(framing, Some(Ok(())));
        let clips = (own, inner.clip);
        let refused = crate::clip_refusals::of(&self.boxes[slot], clips, inner.overlaps, frames);
        crate::kept_clips::InnerClip {
            clip: inner.clip,
            refused,
            rounding: inner.rounding,
        }
    }

    fn scrolls(&self, slot: usize) -> bool {
        self.input.stackings.scrolls(slot)
    }

    fn lane_frame(&self, slot: usize) -> bool {
        self.input
            .placement
            .framed
            .get(slot)
            .copied()
            .unwrap_or(false)
    }
}

/// The slots of `steps`' thumb steps.
fn thumbs(steps: &[Step]) -> Vec<usize> {
    let thumb = |step: &Step| matches!(step, Step::Thumb(_)).then(|| step.index());
    steps.iter().filter_map(thumb).collect()
}

#[path = "kept_order_labels.rs"]
mod labels;

#[path = "kept_order_walks.rs"]
mod walks;
