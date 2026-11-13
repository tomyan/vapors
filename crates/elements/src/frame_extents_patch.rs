//! Patching the extents index (`frame_extents.rs`) for the steps an
//! update painted again, shifted or freed: an entry is overwritten
//! where it sorts to the same position, else taken out and put back,
//! and the reaches after it are made again until one is unchanged.

use super::{Entry, Extents, entry, order};
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
        let Some((frame, top)) = self.at[place] else {
            return false;
        };
        let (now_in, entry) = entry(place, step, boxes);
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
        self.at[place] = Some((frame, entry.top));
        true
    }

    /// Takes the entry at `place` out, if any.
    fn remove(&mut self, place: usize) {
        let Some((frame, top)) = self.at[place].take() else {
            return;
        };
        let index = self
            .frames
            .get_mut(&frame)
            .expect("a frame for every entry");
        let at = index.position(top, place);
        index.items -= index.entries.remove(at).items;
        index.reach_from(at, at);
        if index.entries.is_empty() {
            self.frames.remove(&frame);
        }
    }

    /// Puts `entry`, in frame `frame`, where it sorts.
    fn insert(&mut self, (frame, entry): (usize, Entry)) {
        self.at[entry.place] = Some((frame, entry.top));
        let index = self.frames.entry(frame).or_default();
        let at = index.entries.partition_point(|e| order(e, &entry).is_lt());
        index.entries.insert(at, entry);
        index.items += entry.items;
        index.reach_from(at, at + 1);
    }
}
