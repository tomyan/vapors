//! Patching the extents index (`frame_extents.rs`) for the steps an
//! update painted again, shifted or freed: an entry is overwritten
//! where it sorts to the same position, else taken out and put back,
//! and the reaches after it are made again until one is unchanged.

use super::{Entry, Extents, FrameIndex, entry, missing, order};
use crate::drawn_steps::draws;
use crate::kept_box::KeptBox;
use crate::paint_order::Step;

impl Extents {
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
        let before = at.checked_sub(1).map(|before| &index.entries[before]);
        let fits = before.is_none_or(|before| order(before, &entry).is_lt())
            && (index.entries.get(at + 1)).is_none_or(|after| order(&entry, after).is_lt());
        if now_in != frame || !fits {
            return false;
        }
        index.items = index.items - index.entries[at].items + entry.items;
        index.entries[at] = entry;
        index.reach_from(at, at + 1);
        self.at[place] = Some((frame, entry.top, false));
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
        if tall {
            index.items -= index.tall.remove(index.tall_position(place)).items;
        } else {
            let at = index.position(top, place);
            index.items -= index.entries.remove(at).items;
            index.reach_from(at, at);
        }
        if index.entries.is_empty() && index.tall.is_empty() {
            self.frames.remove(&frame);
        }
    }

    /// Puts `entry`, in frame `frame`, where it sorts.
    fn insert(&mut self, (frame, entry): (usize, Entry)) {
        self.at[entry.place] = Some((frame, entry.top, entry.tall()));
        let index = self.frames.entry(frame).or_default();
        if entry.tall() {
            index.items += entry.items;
            let at = index.tall.partition_point(|e| e.place < entry.place);
            index.tall.insert(at, entry);
            return;
        }
        let at = index.entries.partition_point(|e| order(e, &entry).is_lt());
        index.entries.insert(at, entry);
        index.items += entry.items;
        index.reach_from(at, at + 1);
    }
}

impl FrameIndex {
    /// Where the entry of `place`, at `top`, is, or a panic naming it.
    pub(super) fn position(&self, top: f32, place: usize) -> usize {
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

    /// Each entry's reach from `from` on, stopping at the first entry
    /// from `settled` on whose reach is unchanged: those after it keep
    /// theirs too.
    pub(super) fn reach_from(&mut self, from: usize, settled: usize) {
        let before = from.checked_sub(1).map(|at| self.entries[at].reach);
        let mut reach = before.unwrap_or(f32::NEG_INFINITY);
        for (at, entry) in self.entries.iter_mut().enumerate().skip(from) {
            let now = reach.max(entry.bottom);
            if at >= settled && now.to_bits() == entry.reach.to_bits() {
                return;
            }
            (entry.reach, reach) = (now, now);
        }
    }
}
