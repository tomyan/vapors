//! Each scroll frame's extents index (IU4 PR 5, `docs/plans/iu4.md`
//! section 4): the frame's drawing steps sorted by the top of their
//! items' bounds (frame coordinates), each with its bottom and its
//! reach, the greatest bottom of it and every step before it. The steps
//! that can show in a view are those from the first whose reach passes
//! the view's top to the last whose top is before the view's bottom:
//! two binary searches, and every step entirely above or below the view
//! is outside them. A step with a surface (kept in its batch when
//! culled) or a bound that is not a number spans every view. Patched
//! for the steps painted again, shifted or freed, in place when a step
//! keeps its position; made afresh when paint order is walked or more
//! steps are patched at once than `REBUILD` allows. A debug build
//! compares it with one made afresh after every paint
//! (`kept_paint_check.rs`).

use rustc_hash::FxHashMap;
use vapors_scene::{Item, Rect};

use crate::drawn_steps::DrawnSteps;
use crate::kept_box::KeptBox;
use crate::paint_order::Step;
use crate::scene_view::frame_items;

/// Patching more steps than this, or than a sixteenth of the entries,
/// makes the index afresh: each patch can move a `Vec`'s tail.
const REBUILD: usize = 64;

/// One drawing step in its frame's index.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Entry {
    pub(crate) place: usize,
    pub(crate) step: Step,
    /// How many items the step paints.
    pub(crate) items: u64,
    top: f32,
    bottom: f32,
    /// The greatest `bottom` of this entry and every one before it.
    reach: f32,
}

/// One frame's entries, by top then place, and the items they paint.
#[derive(Debug, Default, PartialEq)]
struct FrameIndex {
    entries: Vec<Entry>,
    items: u64,
}

/// Every frame's index, and where each place's entry is.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct Extents {
    frames: FxHashMap<usize, FrameIndex>,
    /// By place: the frame and top of the step's entry, if it draws.
    at: Vec<Option<(usize, f32)>>,
}

impl Extents {
    /// The index of `drawn`'s steps, `places` steps long in all, from
    /// their parts in `boxes`.
    pub(crate) fn of(drawn: &DrawnSteps, places: usize, boxes: &[Option<KeptBox>]) -> Self {
        let entries = drawn
            .placed()
            .map(|(place, step)| entry(place, step, boxes));
        Self::from_entries(entries, places)
    }

    /// The index of `entries`, each with its frame, `places` steps long.
    fn from_entries(entries: impl Iterator<Item = (usize, Entry)>, places: usize) -> Self {
        let mut at = vec![None; places];
        let mut built: Vec<(usize, FrameIndex)> = Vec::new();
        let mut last = 0;
        for (frame, entry) in entries {
            at[entry.place] = Some((frame, entry.top));
            if built.get(last).is_none_or(|&(was, _)| was != frame) {
                last = built
                    .iter()
                    .position(|&(was, _)| was == frame)
                    .unwrap_or_else(|| {
                        built.push((frame, FrameIndex::default()));
                        built.len() - 1
                    });
            }
            let index = &mut built[last].1;
            index.items += entry.items;
            index.entries.push(entry);
        }
        for (_, index) in &mut built {
            index.entries.sort_unstable_by(order);
            index.reach_from(0, usize::MAX);
        }
        let frames = built.into_iter().collect();
        Self { frames, at }
    }

    /// `self` with `steps`, each with its place, taken out and put back
    /// as their parts in `boxes` now draw; made afresh from `drawn`,
    /// `places` steps long, when there are too many to patch.
    pub(crate) fn patch(
        &mut self,
        steps: &[(usize, Step)],
        (drawn, places): (&DrawnSteps, usize),
        boxes: &[Option<KeptBox>],
    ) {
        let entries: usize = self.frames.values().map(|index| index.entries.len()).sum();
        if steps.len() > REBUILD.max(entries / 16) {
            *self = Self::of(drawn, places, boxes);
            return;
        }
        self.at.resize(places, None);
        for &(place, step) in steps {
            self.repatch(place, step, boxes);
        }
    }

    /// The frames with a drawing step.
    pub(crate) fn frames(&self) -> impl Iterator<Item = usize> + '_ {
        self.frames.keys().copied()
    }

    /// How many items every drawing step paints.
    pub(crate) fn items(&self) -> u64 {
        self.frames.values().map(|index| index.items).sum()
    }

    /// The entries of frame `frame` whose extent meets `view` (frame
    /// coordinates) top to bottom: every step that can show in it.
    pub(crate) fn candidates(&self, frame: usize, view: Rect) -> impl Iterator<Item = &Entry> {
        let entries = self
            .frames
            .get(&frame)
            .map_or(&[][..], |index| &index.entries);
        let first = entries.partition_point(|entry| entry.reach <= view.top);
        let last = entries.partition_point(|entry| entry.top < view.bottom);
        let between = entries.get(first..last).unwrap_or_default();
        between.iter().filter(move |entry| entry.bottom > view.top)
    }
}

impl FrameIndex {
    /// Where the entry of `place`, at `top`, is, or a panic naming it.
    fn position(&self, top: f32, place: usize) -> usize {
        let found = self
            .entries
            .binary_search_by(|entry| entry.top.total_cmp(&top).then(entry.place.cmp(&place)));
        found.unwrap_or_else(|_| {
            panic!(
                "the extents index has no entry for paint step place {place} at top {top}: an \
                 entry was moved without its place's record (frame_extents.rs); reproduce with \
                 `cargo xtask test`"
            )
        })
    }

    /// Each entry's reach from `from` on, stopping at the first entry
    /// from `settled` on whose reach is unchanged: those after it keep
    /// theirs too.
    fn reach_from(&mut self, from: usize, settled: usize) {
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

/// Entries by top, then place.
fn order(a: &Entry, b: &Entry) -> std::cmp::Ordering {
    a.top.total_cmp(&b.top).then(a.place.cmp(&b.place))
}

/// `step`'s entry at `place` from its part in `boxes`, and its frame.
fn entry(place: usize, step: Step, boxes: &[Option<KeptBox>]) -> (usize, Entry) {
    let (frame, items) = frame_items(boxes, step);
    let (mut top, mut bottom, mut count) = (f32::INFINITY, f32::NEG_INFINITY, 0);
    for (item, _) in items {
        let bounds = item.bounds();
        let spans = matches!(item, Item::Surface { .. }) || bounds.top.is_nan();
        let spans = spans || bounds.bottom.is_nan();
        let (item_top, item_bottom) = if spans {
            (f32::NEG_INFINITY, f32::INFINITY)
        } else {
            (bounds.top, bounds.bottom)
        };
        (top, bottom, count) = (top.min(item_top), bottom.max(item_bottom), count + 1);
    }
    let entry = Entry {
        place,
        step,
        items: count,
        top,
        bottom,
        reach: bottom,
    };
    (frame, entry)
}

#[path = "frame_extents_patch.rs"]
mod patch;

#[cfg(test)]
#[path = "frame_extents_tests.rs"]
mod tests;
