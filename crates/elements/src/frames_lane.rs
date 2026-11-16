//! Lane frames (`docs/plans/splice-layout.md`, section 11): a framed
//! lane child is a frame of its own, shifted from the frame its
//! container's content is in by the container's origin there and the
//! child's offset in the lane, and clipped as the container clips its
//! content. Resolved when first read and memoised until the frames are
//! given new lanes, once per update: an insert changes every later
//! child's shift, and only the frames read (drawn, hit) are worked out.

use std::cell::RefCell;

use rustc_hash::FxHashMap;

use super::{FrameAt, Frames};
use prime::Primed;

use crate::layout::{BoxRect, SharedLanes};

/// What a framed lane's children's frames are worked out from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct LaneBase {
    /// The frame the container's content is in.
    pub(crate) parent: usize,
    /// The container's border box top in that frame, CSS pixels.
    pub(crate) origin: f32,
    /// The clip the container gives its content in that frame; `None`
    /// when it scrolls, its scroll frame clipping already, or clips
    /// nothing.
    pub(crate) inner: Option<BoxRect>,
}

/// The lanes the frames read and each framed container's base.
#[derive(Debug, Default)]
pub(crate) struct LaneFrames {
    shared: Option<SharedLanes>,
    bases: FxHashMap<usize, LaneBase>,
    /// The framed containers whose content is in each frame.
    by_parent: FxHashMap<usize, Vec<usize>>,
    memo: RefCell<FxHashMap<usize, FrameAt>>,
    /// The lanes whose children from the one named on started elsewhere
    /// this update (`ChangedSlots::started`), for `prime_started`.
    started: Vec<(usize, usize)>,
    /// The frames `prime_started` worked out (`Primed`), set once an
    /// update: a read of a started row costs an index, not a hash or a
    /// borrow (section 11.2, 4d); the table, sized to the largest primed
    /// slot in an update that started rows, is 16 bytes a slot.
    primed: std::cell::OnceCell<Primed>,
    /// How many times the lanes had changed when they were set
    /// (`Lanes::changes`): the memo holds for that count alone.
    #[cfg(debug_assertions)]
    generation: u64,
    /// Which setting of the lanes this is: unique across every frames
    /// value, so a reader can tell that the frames, set again by every
    /// paint, are the ones it read before (`Painted::frames_epoch`).
    epoch: u64,
    /// Lane frames worked out by a fold of their lane since the lanes
    /// were set, for the tests that the priming holds (`lane_folds`).
    folds: std::cell::Cell<usize>,
}

/// The bases alone: the lanes are the boxes', and the memo is what was
/// read.
impl PartialEq for LaneFrames {
    fn eq(&self, other: &Self) -> bool {
        self.bases == other.bases
    }
}

impl Frames {
    /// The lane bases `from` was set with, carried into frames made
    /// afresh: `set_lanes` then answers whether the bases changed since
    /// the last update, not since these frames were made with none.
    pub(crate) fn keep_lane_bases(&mut self, from: &Frames) {
        self.lanes.bases.clone_from(&from.lanes.bases);
    }

    /// The lane frames from now on: those of the children of the
    /// containers in `bases`, from `shared`; whether any base changed
    /// (a container moved along its axis moves every frame it holds,
    /// with no row placed or started again).
    pub(crate) fn set_lanes(
        &mut self,
        shared: SharedLanes,
        (bases, started): (FxHashMap<usize, LaneBase>, &[(usize, usize)]),
    ) -> bool {
        let changed = self.lanes.bases != bases;
        let mut by_parent: FxHashMap<usize, Vec<usize>> = FxHashMap::default();
        for (&container, base) in &bases {
            by_parent.entry(base.parent).or_default().push(container);
        }
        #[cfg(debug_assertions)]
        let generation = shared.borrow().changes();
        self.lanes = LaneFrames {
            #[cfg(debug_assertions)]
            generation,
            shared: Some(shared),
            bases,
            by_parent,
            memo: RefCell::default(),
            started: started.to_vec(),
            primed: std::cell::OnceCell::new(),
            folds: std::cell::Cell::new(0),
            epoch: next_epoch(),
        };
        changed
    }

    /// How many lane frames a fold of their lane worked out since the
    /// lanes were set: those read and neither primed nor read before.
    #[cfg(test)]
    pub(crate) fn lane_folds(&self) -> usize {
        self.lanes.folds.get()
    }

    /// Lane frame `frame`, when `frame` is a framed lane child,
    /// memoised.
    pub(super) fn lane_at(&self, frame: usize) -> Option<FrameAt> {
        let memoised = self.lanes.memo.borrow().get(&frame).copied();
        if let Some(at) = memoised {
            #[cfg(debug_assertions)]
            self.assert_memo_holds(frame, at);
            return Some(at);
        }
        let at = self.lane_resolved(frame, &|outer| self.at(outer))?;
        self.lanes.folds.set(self.lanes.folds.get() + 1);
        self.lanes.memo.borrow_mut().insert(frame, at);
        Some(at)
    }

    /// Frame `frame` from the memo, or made by `make` and memoised until
    /// the next update; no borrow is held while `make` reads other frames.
    pub(super) fn memoised(&self, frame: usize, make: impl FnOnce() -> FrameAt) -> FrameAt {
        if let Some(&at) = self.lanes.memo.borrow().get(&frame) {
            return at;
        }
        let at = make();
        self.lanes.memo.borrow_mut().insert(frame, at);
        at
    }

    /// Frame `frame` worked out from the lanes as they are, reading and
    /// filling no memo, for the debug checks: a check that filled the
    /// memo would leave a debug build reading lane frames that a release
    /// build works out from lanes changed since.
    pub(crate) fn resolved_afresh(&self, frame: usize) -> FrameAt {
        let outer = |outer| self.resolved_afresh(outer);
        self.lane_resolved(frame, &outer)
            .or_else(|| self.relative_afresh(frame))
            .unwrap_or_else(|| self.at(frame))
    }

    /// Lane frame `frame` worked out, the frame its container's content
    /// is in read through `outer`.
    fn lane_resolved(&self, frame: usize, outer: &dyn Fn(usize) -> FrameAt) -> Option<FrameAt> {
        let (base, offset) = self.lane_place(frame)?;
        Some(self.lane_frame_at(base, offset, outer(base.parent)))
    }

    /// The frame of a child `offset` down a lane at `base`, its
    /// container's content in the frame at `outer`.
    #[allow(clippy::cast_possible_truncation)] // device px, whole, within f32
    pub(super) fn lane_frame_at(&self, base: LaneBase, offset: f64, outer: FrameAt) -> FrameAt {
        self.lane_child_at(self.lane_template(base, outer), base, offset)
    }

    /// The base of the framed lane `frame` is a child of and the
    /// child's offset in it, CSS pixels.
    fn lane_place(&self, frame: usize) -> Option<(LaneBase, f64)> {
        let lanes = self.lanes.shared.as_ref()?.borrow();
        let container = lanes.container_of(frame)?;
        let base = *self.lanes.bases.get(&container)?;
        Some((base, lanes.frame_offset(container, frame)?))
    }
}

/// A lanes setting no other has had.
fn next_epoch() -> u64 {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

#[path = "frames_lane_batch.rs"]
mod batch;
#[path = "frames_lane_meeting.rs"]
mod meeting;
#[path = "frames_lane_seen.rs"]
mod seen;
pub(crate) use meeting::LaneInk;
pub(crate) use seen::LaneSeen;
#[path = "frames_lane_prime.rs"]
mod prime;
