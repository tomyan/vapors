//! Patching the extents index (`frame_extents.rs`) for the steps an
//! update painted again, shifted or freed: an entry is overwritten
//! where it sorts to the same position, else taken out and put back,
//! and the reaches after it are made again until one is unchanged.

use super::{Blocks, Entry, Extents, FrameIndex, entry, missing, order};
use crate::blocks::{IndexCounts, Pos};
use crate::drawn_steps::draws;
use crate::kept_box::KeptBox;
use crate::paint_order::Step;

impl Extents {
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

    /// The totals with `entry` counted out, or a panic naming its place
    /// when either would go below zero: an entry counted out that was
    /// never counted in.
    fn count_out(&mut self, entry: &Entry) {
        let items = self.items.checked_sub(entry.items);
        let short = self.short.checked_sub(usize::from(!entry.tall()));
        let (Some(items), Some(short)) = (items, short) else {
            panic!(
                "the extents index's totals ({} items, {} short entries) go below zero taking \
                 out the entry of place {} ({} items): it was not counted in \
                 (frame_extents_patch.rs `count_out`); reproduce with `cargo xtask test`",
                self.items, self.short, entry.place, entry.items
            );
        };
        (self.items, self.short) = (items, short);
    }

    /// The entry at `place` made again from `step`'s part in `boxes`:
    /// overwritten where it sorts to the same position, else taken out
    /// and put back; taken out when the step no longer draws.
    pub(super) fn repatch(&mut self, place: usize, step: Step, boxes: &[Option<KeptBox>]) {
        let was = self.at[place];
        let draws = draws(boxes, step);
        if was.is_some() && draws && self.replaced(place, step, boxes) {
            return;
        }
        self.remove(place);
        if draws {
            self.insert(entry(place, step, boxes));
        }
    }

    /// True when `step`'s new entry at `place` is in the frame of its
    /// old one and sorts to the same position, which it then takes.
    fn replaced(&mut self, place: usize, step: Step, boxes: &[Option<KeptBox>]) -> bool {
        let Some((frame, top, false)) = self.at[place] else {
            return false;
        };
        let (now_in, entry) = entry(place, step, boxes);
        if entry.tall() {
            return false;
        }
        let index = self
            .frames
            .get_mut(&frame)
            .expect("a frame for every entry");
        let at = index.position(top, place);
        if now_in != frame || !index.fits(at, &entry) {
            return false;
        }
        let was = index.entries.get_mut(at).unwrap_or_else(|| missing(place));
        let was = std::mem::replace(was, entry);
        index.reach_from(at, 1);
        self.at[place] = Some((frame, entry.top, false));
        self.count_out(&was);
        self.count_in(&entry);
        true
    }

    /// Takes the entry at `place` out, if any.
    fn remove(&mut self, place: usize) {
        let Some((frame, top, tall)) = self.at[place].take() else {
            return;
        };
        let index = self
            .frames
            .get_mut(&frame)
            .expect("a frame for every entry");
        let blocks_were = index.entries.block_count();
        let gone = index.taken(place, (top, tall));
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
        self.at[entry.place] = Some((frame, entry.top, entry.tall()));
        self.count_in(&entry);
        self.span_in(frame, &entry);
        let index = self.frames.entry(frame).or_default();
        if entry.tall() {
            let at = index.tall.partition_point(|e| e.place < entry.place);
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

impl FrameIndex {
    /// The entry of `place`, at `top`, tall or not, taken out.
    fn taken(&mut self, place: usize, (top, tall): (f32, bool)) -> Entry {
        if tall {
            return self.tall.remove(self.tall_position(place));
        }
        let (gone, next) = self.entries.remove(self.position(top, place));
        self.reach_from(next, 0);
        gone
    }

    /// Where the entry of `place`, at `top`, is, or a panic naming it.
    pub(super) fn position(&self, top: f32, place: usize) -> Pos {
        let found = self
            .entries
            .binary_search_by(|entry| entry.top.total_cmp(&top).then(entry.place.cmp(&place)));
        found.unwrap_or_else(|_| missing(place))
    }

    /// Where the tall entry of `place` is, or a panic naming it.
    pub(super) fn tall_position(&self, place: usize) -> usize {
        let found = self.tall.iter().position(|entry| entry.place == place);
        found.unwrap_or_else(|| missing(place))
    }

    /// True when `entry` sorts between the entries around `at`, so it
    /// can take the place of the one there.
    fn fits(&self, at: Pos, entry: &Entry) -> bool {
        let before = self.entries.prev(at).and_then(|at| self.entries.get(at));
        let after = self.entries.iter_from(at).nth(1);
        before.is_none_or(|before| order(before, entry).is_lt())
            && after.is_none_or(|after| order(entry, after).is_lt())
    }

    /// Each entry's reach from `from` on: the first `settled` of them
    /// set whatever they were, then stopping at the first whose reach
    /// is unchanged, as those after it keep theirs too.
    pub(super) fn reach_from(&mut self, from: Pos, settled: usize) {
        let before = self.entries.prev(from).and_then(|at| self.entries.get(at));
        let mut reach = before.map_or(f32::NEG_INFINITY, |entry| entry.reach);
        for (count, entry) in self.entries.iter_mut_from(from).enumerate() {
            let now = reach.max(entry.bottom);
            if count >= settled && now.to_bits() == entry.reach.to_bits() {
                return;
            }
            (entry.reach, reach) = (now, now);
        }
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

impl FrameIndex {
    /// Takes `entries`, in any order, as the short entries, each with
    /// its reach.
    pub(super) fn sorted(&mut self, mut entries: Vec<Entry>) {
        entries.sort_unstable_by(order);
        self.entries = Blocks::from_sorted(entries);
        self.reach_from(Pos::default(), usize::MAX);
    }
}
