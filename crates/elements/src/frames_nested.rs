//! Each scroll frame's index of the scroll containers in it (IU4 PR 5's
//! M2, `docs/plans/iu4.md`): each container's inner clip, device pixels
//! in the frame's coordinates, which bounds where anything in the
//! container's own frame can show. A view then visits only the frames
//! whose container's clip meets it, not every frame. As the extents
//! index (`frame_extents.rs`): sorted by top, then slot, with the
//! running maximum of the bottoms; clips taller than [`TALL`], and
//! containers with no clip, are kept apart and visited on every view.

use vapors_scene::Rect;

use super::{Frames, PAGE};

/// Clips taller than this many device pixels are kept apart, so one
/// cannot raise every later entry's reach.
const TALL: f32 = 1024.0;

/// One scroll container in its frame's index.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Nested {
    slot: usize,
    clip: Rect,
    /// The largest bottom of this entry and every one before it.
    reach: f32,
}

/// The scroll containers in one frame, by their clips.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct NestedIndex {
    entries: Vec<Nested>,
    /// Tall or unclipped containers, by slot.
    tall: Vec<usize>,
}

impl NestedIndex {
    /// The index of `nested`, each container with its clip.
    pub(crate) fn of(nested: impl Iterator<Item = (usize, Option<Rect>)>) -> Self {
        let mut index = Self::default();
        for (slot, clip) in nested {
            match short(clip) {
                Some(clip) => index.entries.push(Nested::new(slot, clip)),
                None => index.tall.push(slot),
            }
        }
        index.entries.sort_unstable_by(order);
        index.tall.sort_unstable();
        index.reach_from(0, usize::MAX);
        index
    }

    /// The containers whose clip meets `view`, frame coordinates, and
    /// the tall and unclipped ones.
    pub(crate) fn meeting(&self, view: Rect) -> impl Iterator<Item = usize> + '_ {
        let first = self
            .entries
            .partition_point(|entry| entry.reach <= view.top);
        let last = self
            .entries
            .partition_point(|entry| entry.clip.top < view.bottom);
        let between = self.entries.get(first..last).unwrap_or_default();
        let meets = between
            .iter()
            .filter(move |entry| !entry.clip.is_disjoint(&view));
        self.tall
            .iter()
            .copied()
            .chain(meets.map(|entry| entry.slot))
    }

    /// Puts container `slot` with clip `clip` where it sorts.
    pub(crate) fn insert(&mut self, slot: usize, clip: Option<Rect>) {
        let Some(clip) = short(clip) else {
            let at = self.tall.partition_point(|&tall| tall < slot);
            self.tall.insert(at, slot);
            return;
        };
        let entry = Nested::new(slot, clip);
        let at = self.entries.partition_point(|e| order(e, &entry).is_lt());
        self.entries.insert(at, entry);
        self.reach_from(at, at + 1);
    }

    /// Takes container `slot`, entered with clip `clip`, out; a panic
    /// naming it when it is not there.
    pub(crate) fn remove(&mut self, slot: usize, clip: Option<Rect>) {
        let Some(clip) = short(clip) else {
            self.tall
                .remove(position(self.tall.binary_search(&slot), slot));
            return;
        };
        let key = Nested::new(slot, clip);
        let at = position(self.entries.binary_search_by(|e| order(e, &key)), slot);
        self.entries.remove(at);
        self.reach_from(at, at);
    }

    /// Each entry's reach from `from` on, stopping at the first from
    /// `settled` on whose reach is unchanged.
    fn reach_from(&mut self, from: usize, settled: usize) {
        let before = from.checked_sub(1).map(|at| self.entries[at].reach);
        let mut reach = before.unwrap_or(f32::NEG_INFINITY);
        for (at, entry) in self.entries.iter_mut().enumerate().skip(from) {
            let now = reach.max(entry.clip.bottom);
            if at >= settled && now.to_bits() == entry.reach.to_bits() {
                return;
            }
            (entry.reach, reach) = (now, now);
        }
    }
}

impl Nested {
    fn new(slot: usize, clip: Rect) -> Self {
        Self {
            slot,
            clip,
            reach: 0.0,
        }
    }
}

/// `clip` as a short entry's, its top -0.0 made 0.0 so that the order
/// (`total_cmp`) finds it whichever zero it was entered or taken out
/// with; `None` when it is kept apart.
fn short(clip: Option<Rect>) -> Option<Rect> {
    let clip = clip.filter(|clip| !is_tall(clip))?;
    Some(Rect::new(
        clip.left,
        clip.top + 0.0,
        clip.right,
        clip.bottom,
    ))
}

/// Where a search found `slot`, or a panic naming it.
fn position(found: Result<usize, usize>, slot: usize) -> usize {
    found.unwrap_or_else(|_| missing(slot))
}

/// Whether `clip` is kept apart: taller than [`TALL`], or with an edge
/// that is not a number.
fn is_tall(clip: &Rect) -> bool {
    let height = clip.bottom - clip.top;
    height > TALL || height.is_nan()
}

/// Entries by top, then slot.
fn order(a: &Nested, b: &Nested) -> std::cmp::Ordering {
    a.clip.top.total_cmp(&b.clip.top).then(a.slot.cmp(&b.slot))
}

impl Frames {
    /// The frames whose items can show in `view`, device pixels on the
    /// page: the page's, each frame whose container's clip meets the
    /// view in the frame that container is in, found through each
    /// frame's index of its containers (`frames_nested.rs`), and each
    /// lane frame whose child meets the view in its lane's frame
    /// (`frames_lane.rs`).
    pub(crate) fn meeting(&self, view: Rect) -> Vec<usize> {
        let mut found = Vec::new();
        let mut open = vec![PAGE];
        while let Some(frame) = open.pop() {
            found.push(frame);
            if let Some(index) = self.index.get(&frame) {
                open.extend(index.meeting(self.view_in(frame, view)));
            }
            open.extend(self.lane_frames_meeting(frame, view));
        }
        found
    }
}

fn missing(slot: usize) -> ! {
    panic!(
        "scroll container {slot} is not in its frame's index of nested containers: a frame's \
         clip was entered under another value than it is taken out with (frames_update.rs); \
         reproduce with `cargo xtask test`"
    )
}

#[cfg(test)]
#[path = "frames_nested_tests.rs"]
mod tests;
