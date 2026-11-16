//! Batching in scroll frames (IU4 4c, `docs/plans/iu4.md` section 3,
//! "Batching" and "Culling"): each item stays in its part's frame's
//! coordinates under its clip inside the frame, and is culled against
//! the view as it falls in that frame, cut to the frame's clip.
//! Consecutive items share a batch within one frame, or across the rows
//! of one framed lane (`docs/plans/splice-layout.md` section 11.2, 4f):
//! such a batch names the lane, each item its row, whose offset the
//! painter adds per instance. A lane's run is split at
//! `vapors_scene::ROWS_PER_BATCH` rows (`framed_batches_rows.rs`).

use vapors_scene::{Batched, Item, Rect};

use crate::frames::Frames;
use crate::frames_draw::{BatchFrame, Framing};
use crate::kept_box::KeptBox;
use crate::paint_order::Step;
use crate::scene_view::frame_items;

#[path = "framed_batches_arena.rs"]
mod arena;
pub(crate) use arena::NO_ROW;
pub use arena::{BatchRows, RowsOfBatches};
#[path = "framed_batches_rows.rs"]
mod rows;
#[path = "framed_batches_slots.rs"]
mod slots;
use rows::rows_batched;
pub(crate) use slots::RowSlots;

/// The batches of a view, each with where its items are and, for a
/// batch over the rows of a lane, which row each item is in.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FramedBatched {
    /// The batches, in painting order.
    pub batched: Batched,
    /// Each batch's frame or lane, by its place in `batched.batches`.
    pub frames: Vec<BatchFrame>,
    /// Each batch's rows, by its place: none for a batch in one frame.
    pub rows: RowsOfBatches,
}

/// The batches of `steps`' items (each entry's step by `step`) for a
/// target whose drawable area is `view` (device pixels on the page): each
/// run of consecutive steps in one frame, or in the rows of one lane,
/// batched apart.
pub(crate) fn framed_batches<T>(
    steps: &[T],
    step: impl Fn(&T) -> Step,
    sources: Sources<'_>,
    (view, slots): (Rect, &mut RowSlots),
) -> FramedBatched {
    let resolved = resolved(steps.iter().map(step), sources);
    let mut batches = FramedBatched::default();
    for run in resolved.chunk_by(|a, b| a.group == b.group) {
        batches.add_run(run, sources, (view, slots));
    }
    batches
}

/// What batching reads of the scene: the kept boxes and the frames.
pub(crate) type Sources<'a> = (&'a [Option<KeptBox>], &'a Frames);

/// A step with its frame and what it is batched with: the frame, or its
/// lane's rows. Worked out once a step: a lane frame's lookups walk the
/// lane's tree.
#[derive(Clone, Copy)]
pub(crate) struct Resolved {
    pub(crate) step: Step,
    pub(crate) frame: usize,
    pub(crate) group: BatchFrame,
}

/// Each of `steps` resolved, a frame's group looked up again only when
/// the frame changes from the step before. A row's steps mostly run
/// together; a positioned descendant's paint later, and the lookup is
/// then made again. A step in a lane row that paints a shape is batched
/// in the row's own frame: the shape and mesh pipelines take no table
/// of row offsets (option (b) of the fold; the flat scene's transform
/// table, F2, retires the per-row draws this costs).
fn resolved(steps: impl Iterator<Item = Step>, (boxes, frames): Sources<'_>) -> Vec<Resolved> {
    let mut last = LastSeen::default();
    let resolve = |step| {
        let (frame, mut items) = frame_items(boxes, step);
        let group = last.get(frame, || {
            frames
                .lane_container_of(frame)
                .map_or_else(|| BatchFrame::frame(frame), BatchFrame::lane)
        });
        let in_lane = matches!(group.0, Framing::Lane(_));
        let group = if in_lane && items.any(|(item, _)| matches!(item, Item::Shape(_))) {
            BatchFrame::frame(frame)
        } else {
            group
        };
        Resolved { step, frame, group }
    };
    steps.map(resolve).collect()
}

/// The value made for the last key asked for: made again when the key
/// changes. For keys that mostly repeat (a row's frame, step after step).
pub(crate) struct LastSeen<K, V>(Option<(K, V)>);

impl<K, V> Default for LastSeen<K, V> {
    fn default() -> Self {
        Self(None)
    }
}

impl<K: PartialEq + Copy, V: Copy> LastSeen<K, V> {
    /// The value for `key`: the last one when `key` was the last asked
    /// for, else `make()`'s.
    pub(crate) fn get(&mut self, key: K, make: impl FnOnce() -> V) -> V {
        match self.0 {
            Some((last, value)) if last == key => value,
            _ => {
                let value = make();
                self.0 = Some((key, value));
                value
            }
        }
    }
}

impl FramedBatched {
    /// The batches of `run`, steps of one group, added.
    fn add_run(
        &mut self,
        run: &[Resolved],
        sources: Sources<'_>,
        (view, slots): (Rect, &mut RowSlots),
    ) {
        let group = run[0].group;
        match group.0 {
            Framing::Lane(container) => {
                let lane = slots.lane(container);
                for (part, rows) in rows_batched(run, (container, sources), (view, lane)) {
                    self.extend(group, (part, Some(rows)));
                }
            }
            Framing::Frame(frame) => {
                let (boxes, frames) = sources;
                let items = run.iter().flat_map(|at| frame_items(boxes, at.step).1);
                let part = vapors_scene::batched(items, frames.view_in(frame, view));
                self.extend(group, (part, None));
            }
        }
    }

    /// `part`'s batches added, each at `frame`, with their rows (`None`
    /// for batches in one frame).
    fn extend(&mut self, frame: BatchFrame, (part, rows): (Batched, Option<RowsOfBatches>)) {
        let count = part.batches.len();
        self.batched.culled += part.culled;
        self.frames.extend(std::iter::repeat_n(frame, count));
        match rows {
            Some(rows) => self.rows.append(&rows),
            None => self.rows.add_unrowed(count),
        }
        self.batched.batches.extend(part.batches);
    }
}

/// How many items `step` paints when every one is culled in `view`
/// (device pixels on the page) and none is a surface (a culled surface
/// keeps its place in a batch); `None` when one shows.
pub(crate) fn hidden_items(step: Step, (boxes, frames): Sources<'_>, view: Rect) -> Option<u64> {
    let frame = frame_items(boxes, step).0;
    hidden_in(step, (boxes, frame), frames.view_in(frame, view))
}

/// As [`hidden_items`], for a step in `frame`, `in_frame` the view in that
/// frame's coordinates and cut to its clip: what a search frame by frame
/// works out once a frame (`KeptBatches::of`).
///
/// # Panics
///
/// In a debug build, when `step` is not in `frame`: culled in another
/// frame's view, it would be shown or hidden wrongly.
pub(crate) fn hidden_in(
    step: Step,
    (boxes, frame): (&[Option<KeptBox>], usize),
    in_frame: Rect,
) -> Option<u64> {
    let (step_frame, items) = frame_items(boxes, step);
    debug_assert_eq!(
        step_frame, frame,
        "a step searched in frame {frame} is in frame {step_frame}: the extents index holds it \
         under the wrong frame (frame_extents.rs); reproduce with `cargo xtask test`"
    );
    let view = in_frame;
    let mut count = 0;
    for (item, clip) in items {
        let surface = matches!(item, Item::Surface { .. });
        if surface || !vapors_scene::culled(&item, clip, view) {
            return None;
        }
        count += 1;
    }
    Some(count)
}
