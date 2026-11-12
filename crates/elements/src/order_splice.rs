//! Whether a shape change can be spliced into the kept paint order, and
//! the splice (`docs/plans/splice.md`, section (c)), decided on each
//! update from what `ChangedSlots` holds and each box's `Stacking`,
//! never guessed. When one slot's children are its kept children with
//! runs of new slots inserted and runs of freed slots removed, every
//! inserted and freed box has the default stacking (in flow, no
//! z-index, not scrolling) and no rebuilt box's stacking changed, each
//! inserted run's box steps go into its parent's flow run, before the
//! next kept in-flow sibling's or after the last in-flow box of the
//! previous one, and its outline steps into the enclosing frame's at
//! the matching place; the freed slots' steps are taken out. Anything
//! else is declined, and the whole tree is walked (`kept_order.rs`),
//! which is the exact answer. A debug build compares every spliced
//! order with one walked afresh (`kept_order_check.rs`).

use std::ops::Range;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::changed_slots::ChangedSlots;
use crate::layout::{Content, LayoutBox};
use crate::paint_order::{Stacking, Stackings, Step};

/// Where a step is in the kept order, before the splice.
pub(crate) type Place<'a> = &'a dyn Fn(Step) -> Option<usize>;

/// One edit of the kept steps: `removed` steps taken out at `at`, a
/// place in the order before any edit, and `inserted` put in there.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Edit {
    pub(crate) at: usize,
    pub(crate) removed: usize,
    pub(crate) inserted: Vec<Step>,
}

/// A shape change as a splice: the edits of the steps, and the new
/// slots each with its clip parent, parents first.
#[derive(Debug, Default)]
pub(crate) struct Splice {
    pub(crate) edits: Vec<Edit>,
    pub(crate) added: Vec<(usize, usize)>,
}

/// The change cannot be spliced: the whole tree is walked.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Declined;

/// The splice of the `changed` slots of `boxes`, or `Declined`.
pub(crate) fn plan(
    boxes: &[LayoutBox],
    stackings: &Stackings,
    changed: &ChangedSlots,
    place: Place<'_>,
) -> Result<Splice, Declined> {
    if changed.stackings.restacked || changed.stackings.freed_stacked {
        return Err(Declined);
    }
    let [(owner, old)] = changed.reshaped.as_slice() else {
        return Err(Declined);
    };
    let children = Children {
        owner: *owner,
        new: &boxes[*owner].children,
        old: old
            .iter()
            .enumerate()
            .map(|(rank, &slot)| (slot, rank))
            .collect(),
    };
    let mut splice = Splice {
        edits: removals(changed, place),
        added: Vec::new(),
    };
    for run in children.inserted_runs(boxes)? {
        let below = in_flow_subtrees(boxes, stackings, children.owner, &children.new[run.clone()])?;
        let (boxes_at, outlines_at) = children.anchors(boxes, stackings, &run, place)?;
        let steps =
            |step: fn(usize) -> Step| below.iter().map(move |&(slot, _)| step(slot)).collect();
        splice.edits.push(Edit {
            at: boxes_at,
            removed: 0,
            inserted: steps(Step::Box),
        });
        splice.edits.push(Edit {
            at: outlines_at,
            removed: 0,
            inserted: steps(Step::Outline),
        });
        splice.added.extend(below);
    }
    Ok(splice)
}

/// The reshaped slot's children now, and the place each slot had among
/// the children the slot held.
struct Children<'a> {
    owner: usize,
    new: &'a [usize],
    old: FxHashMap<usize, usize>,
}

impl Children<'_> {
    /// The runs of `new` not in `old`, as ranges of `new`; `Declined`
    /// when the slots in both are not in the same order, or a slot of
    /// `old` not in `new` still holds a box (it moved to another parent).
    fn inserted_runs(&self, boxes: &[LayoutBox]) -> Result<Vec<Range<usize>>, Declined> {
        let now: FxHashSet<usize> = self.new.iter().copied().collect();
        let moved =
            |slot: &usize| !now.contains(slot) && !matches!(boxes[*slot].content, Content::Vacant);
        if self.old.keys().any(moved) {
            return Err(Declined);
        }
        let mut runs: Vec<Range<usize>> = Vec::new();
        let mut last_rank = None;
        for (at, slot) in self.new.iter().enumerate() {
            match (self.old.get(slot), runs.last_mut()) {
                (Some(rank), _) if last_rank < Some(rank) => last_rank = Some(rank),
                (Some(_), _) => return Err(Declined),
                (None, Some(run)) if run.end == at => run.end = at + 1,
                (None, _) => runs.push(at..at + 1),
            }
        }
        Ok(runs)
    }

    fn kept(&self, slot: usize) -> bool {
        self.old.contains_key(&slot)
    }
}

/// The freed slots' box and outline steps taken out: one edit per run
/// of consecutive places.
fn removals(changed: &ChangedSlots, place: Place<'_>) -> Vec<Edit> {
    let steps = |&(slot, _): &(usize, _)| [Step::Box(slot), Step::Outline(slot)];
    let mut places: Vec<usize> = changed
        .freed
        .iter()
        .flat_map(steps)
        .map(|step| placed(place, step))
        .collect();
    places.sort_unstable();
    let mut edits: Vec<Edit> = Vec::new();
    for at in places {
        match edits.last_mut() {
            Some(edit) if edit.at + edit.removed == at => edit.removed += 1,
            _ => edits.push(Edit {
                at,
                removed: 1,
                inserted: Vec::new(),
            }),
        }
    }
    edits
}

/// `roots` and the boxes below them in tree order, each with its
/// parent (`parent` for the roots); `Declined` when one has a stacking
/// other than the default.
fn in_flow_subtrees(
    boxes: &[LayoutBox],
    stackings: &Stackings,
    parent: usize,
    roots: &[usize],
) -> Result<Vec<(usize, usize)>, Declined> {
    let mut open: Vec<(usize, usize)> = roots.iter().rev().map(|&slot| (slot, parent)).collect();
    let mut below = Vec::new();
    while let Some((slot, parent)) = open.pop() {
        if stackings.get(slot) != Stacking::default() {
            return Err(Declined);
        }
        below.push((slot, parent));
        open.extend(
            boxes[slot]
                .children
                .iter()
                .rev()
                .map(|&child| (child, slot)),
        );
    }
    Ok(below)
}

/// The place of `step`, which every kept box's box and outline step has.
fn placed(place: Place<'_>, step: Step) -> usize {
    place(step).unwrap_or_else(|| {
        panic!(
            "{step:?} has no place in the kept paint order: the slot-to-step index \
             (drawn_steps.rs) is not from the kept steps (kept_order.rs); reproduce with \
             `cargo xtask test`"
        )
    })
}

#[path = "order_splice_anchors.rs"]
mod anchors;
