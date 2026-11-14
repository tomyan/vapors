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
//! (`kept_paint_check.rs`). The short entries are kept in blocks
//! (`blocks.rs`): one put in or taken out moves at most a block's tail.

use rustc_hash::FxHashMap;
use vapors_scene::Item;

use crate::blocks::Blocks;
use crate::drawn_steps::DrawnSteps;
use crate::kept_box::KeptBox;
use crate::paint_order::Step;
use crate::scene_view::frame_items;

/// Patching more steps than this, or than a sixteenth of the entries,
/// makes the index afresh: each patch is a search and a block's move.
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

/// Steps taller than this many device pixels (and those that span every
/// view) are kept apart from the others and tested on every view, so
/// one tall step cannot raise every later step's reach: a short entry's
/// reach passes its top by at most this, and the scan from the first
/// whose reach passes the view's top visits only short entries whose top
/// is within this above the view. The rows page has 3 such steps.
const TALL: f32 = 1024.0;

/// One frame's entries: the short by top then place, the tall by place;
/// and the items they paint.
#[derive(Debug, Default, PartialEq)]
struct FrameIndex {
    entries: Blocks<Entry>,
    tall: Vec<Entry>,
    items: u64,
}

/// Where a place's entry is: its frame, its top, and whether it is tall.
type At = (usize, f32, bool);

/// Every frame's index, and where each place's entry is.
#[derive(Debug, Default)]
pub(crate) struct Extents {
    frames: FxHashMap<usize, FrameIndex>,
    /// By place: where the step's entry is, if it draws.
    at: Vec<Option<At>>,
    /// Times a patch made the index afresh since the last take, for
    /// `UpdateCounts`.
    rebuilds: usize,
    /// Entries the patches moved since the last take, and the blocks
    /// every frame's entries hold: running totals, so no update sums
    /// over the frames (`IndexCounts`).
    moved: usize,
    blocks: usize,
    /// The frames holding a step that spans every view (a surface, or
    /// bounds that are not numbers), each with how many: searched on
    /// every view, whatever their container's clip (`kept_batches.rs`).
    spanning: FxHashMap<usize, usize>,
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
        let mut extents = Self {
            frames: FxHashMap::default(),
            at: vec![None; places],
            rebuilds: 0,
            moved: 0,
            blocks: 0,
            spanning: FxHashMap::default(),
        };
        let mut short: FxHashMap<usize, Vec<Entry>> = FxHashMap::default();
        for (frame, entry) in entries {
            extents.at[entry.place] = Some((frame, entry.top, entry.tall()));
            extents.span_in(frame, &entry);
            let index = extents.frames.entry(frame).or_default();
            index.items += entry.items;
            if entry.tall() {
                index.tall.push(entry);
            } else {
                short.entry(frame).or_default().push(entry);
            }
        }
        for (frame, entries) in short {
            let index = extents.frames.entry(frame).or_default();
            index.sorted(entries);
            extents.blocks += index.entries.block_count();
        }
        extents
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
            let rebuilds = self.rebuilds + 1;
            *self = Self::of(drawn, places, boxes);
            self.rebuilds = rebuilds;
            return;
        }
        assert_eq!(
            places,
            self.at.len(),
            "the extents index holds {} paint places and paint order has {places}: a change of \
             paint order reached the patch without a walk, which makes the index afresh \
             (paint_partial.rs drawn_updated); reproduce with `cargo xtask test`",
            self.at.len()
        );
        for &(place, step) in steps {
            self.repatch(place, step, boxes);
        }
    }
}

impl Entry {
    /// True when the step is kept apart as tall (`TALL`).
    fn tall(&self) -> bool {
        self.bottom - self.top > TALL
    }
}

/// A panic naming `place`, whose entry is not where its record says.
pub(super) fn missing(place: usize) -> ! {
    panic!(
        "the extents index has no entry for paint step place {place} where its record says: an \
         entry was moved without its place's record (frame_extents.rs); reproduce with `cargo \
         xtask test`"
    )
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
#[path = "frame_extents_query.rs"]
mod query;

#[cfg(test)]
#[path = "frame_extents_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "frame_extents_tall_tests.rs"]
mod tall_tests;

#[cfg(test)]
#[path = "frame_extents_moves_tests.rs"]
mod moves_tests;
