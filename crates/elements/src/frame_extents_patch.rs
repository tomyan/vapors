//! Patching the extents index (`frame_extents.rs`) for the steps an
//! update painted again, shifted or freed: an entry is overwritten
//! where it sorts to the same position, else taken out and put back,
//! and the reaches after it are made again until one is unchanged.

use super::{Entry, Extents, REBUILD, entry, missing, order};
use crate::blocks::IndexCounts;
use crate::drawn_steps::DrawnSteps;
use crate::drawn_steps::draws;
use crate::kept_box::KeptBox;
use crate::paint_labels::Label;
use crate::paint_order::Step;

impl Extents {
    /// `self` with `steps`, each with its label, taken out and put back
    /// as their parts in `boxes` now draw; made afresh from `drawn`,
    /// `slots` steps long, when there are too many to patch.
    pub(crate) fn patch(
        &mut self,
        steps: &[(Label, Step)],
        (drawn, slots): (&DrawnSteps, usize),
        boxes: &[Option<KeptBox>],
    ) {
        if steps.len() > REBUILD.max(self.short / 16) {
            let rebuilds = self.rebuilds + 1;
            *self = Self::of(drawn, slots, boxes);
            self.rebuilds = rebuilds;
            return;
        }
        assert_eq!(
            slots,
            self.at.len(),
            "the extents index holds {} slots and the boxes are {slots}: a change of paint \
             order reached the patch without a walk or splice, which makes the index afresh \
             (paint_partial.rs drawn_updated); reproduce with `cargo xtask test`",
            self.at.len()
        );
        for &(label, step) in steps {
            self.repatch(label, step, boxes);
        }
    }

    /// What the patches since the last take did (`blocks.rs`
    /// `IndexCounts`): entries moved, rebuilds, and the blocks held.
    pub(crate) fn take_counts(&mut self) -> IndexCounts {
        IndexCounts {
            moved: std::mem::take(&mut self.moved),
            rebuilds: std::mem::take(&mut self.rebuilds),
            blocks: self.blocks,
        }
    }

    /// Adds what frame `frame`'s entries just did to the running
    /// totals: the entries moved, and its blocks against `blocks_were`.
    fn noted(&mut self, frame: usize, blocks_were: usize) {
        let index = self.frames.get_mut(&frame).expect("a frame just patched");
        self.moved += index.entries.take_moved();
        self.blocks = self.blocks + index.entries.block_count() - blocks_were;
    }

    /// The totals with `entry` counted in.
    pub(super) fn count_in(&mut self, entry: &Entry) {
        self.items += entry.items;
        self.short += usize::from(!entry.tall());
    }

    /// The totals with `entry` counted out, or a panic naming its label
    /// when either would go below zero: an entry counted out that was
    /// never counted in.
    fn count_out(&mut self, entry: &Entry) {
        let items = self.items.checked_sub(entry.items);
        let short = self.short.checked_sub(usize::from(!entry.tall()));
        let (Some(items), Some(short)) = (items, short) else {
            panic!(
                "the extents index's totals ({} items, {} short entries) go below zero taking \
                 out the entry of label {} ({} items): it was not counted in \
                 (frame_extents_patch.rs `count_out`); reproduce with `cargo xtask test`",
                self.items, self.short, entry.label, entry.items
            );
        };
        (self.items, self.short) = (items, short);
    }

    /// The entry at `label` made again from `step`'s part in `boxes`:
    /// overwritten where it sorts to the same position, else taken out
    /// and put back; taken out when the step no longer draws.
    pub(super) fn repatch(&mut self, label: Label, step: Step, boxes: &[Option<KeptBox>]) {
        let was = self.at_of(step);
        let draws = draws(boxes, step);
        if was.is_some() && draws && self.replaced(label, step, boxes) {
            return;
        }
        self.remove(label, step);
        if draws {
            self.insert(entry(label, step, boxes));
        }
    }

    /// True when `step`'s new entry at `label` is in the frame of its
    /// old one and sorts to the same position, which it then takes.
    fn replaced(&mut self, label: Label, step: Step, boxes: &[Option<KeptBox>]) -> bool {
        let Some((frame, top, false)) = self.at_of(step) else {
            return false;
        };
        let (now_in, entry) = entry(label, step, boxes);
        if entry.tall() {
            return false;
        }
        let index = self
            .frames
            .get_mut(&frame)
            .expect("a frame for every entry");
        let at = index.position(top, label);
        if now_in != frame || !index.fits(at, &entry) {
            return false;
        }
        let was = index.entries.get_mut(at).unwrap_or_else(|| missing(label));
        let was = std::mem::replace(was, entry);
        index.reach_from(at, 1);
        *self.at_mut(step) = Some((frame, entry.top, false));
        self.count_out(&was);
        self.count_in(&entry);
        true
    }

    /// Takes the entry at `label` out, if any.
    pub(super) fn remove(&mut self, label: Label, step: Step) {
        let Some((frame, top, tall)) = self.at_mut(step).take() else {
            return;
        };
        let index = self
            .frames
            .get_mut(&frame)
            .expect("a frame for every entry");
        let blocks_were = index.entries.block_count();
        let gone = index.taken(label, (top, tall));
        let empty = index.entries.is_empty() && index.tall.is_empty();
        self.count_out(&gone);
        self.span_out(frame, &gone);
        self.noted(frame, blocks_were);
        if empty {
            self.frames.remove(&frame);
        }
    }

    /// Puts `entry`, in frame `frame`, where it sorts.
    fn insert(&mut self, (frame, entry): (usize, Entry)) {
        *self.at_mut(entry.step) = Some((frame, entry.top, entry.tall()));
        self.count_in(&entry);
        self.span_in(frame, &entry);
        let index = self.frames.entry(frame).or_default();
        if entry.tall() {
            let at = index.tall.partition_point(|e| e.label < entry.label);
            index.tall.insert(at, entry);
            return;
        }
        let blocks_were = index.entries.block_count();
        let at = index.entries.partition_point(|e| order(e, &entry).is_lt());
        let at = index.entries.insert(at, entry);
        index.reach_from(at, 1);
        self.noted(frame, blocks_were);
    }
}

impl PartialEq for Extents {
    /// The same entries in the same frames, and the same totals over
    /// them, whatever the patches did.
    fn eq(&self, other: &Self) -> bool {
        let totals = (self.items, self.short) == (other.items, other.short);
        self.frames == other.frames
            && self.at == other.at
            && totals
            && self.spanning == other.spanning
    }
}
