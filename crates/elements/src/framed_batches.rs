//! Batching in scroll frames (IU4 4c, `docs/plans/iu4.md` section 3,
//! "Batching" and "Culling"): each item stays in its part's frame's
//! coordinates under its clip inside the frame, and is culled against
//! the view as it falls in that frame, cut to the frame's clip.
//! Consecutive items share a batch within one frame, or across the rows
//! of one framed lane (`docs/plans/splice-layout.md` section 11.2, 4f):
//! such a batch names the lane, each item its row, whose offset the
//! painter adds per instance. A lane's run is split at
//! `vapors_scene::ROWS_PER_BATCH` rows (`framed_batches_rows.rs`).

use rustc_hash::FxHashMap;

use vapors_scene::{Batched, Item, Rect};

use crate::frames::Frames;
use crate::frames_draw::{BatchFrame, Framing};
use crate::kept_box::KeptBox;
use crate::paint_order::Step;
use crate::scene_view::frame_items;

#[path = "framed_batches_rows.rs"]
mod rows;
use rows::rows_batched;

/// The batches of a view, each with where its items are and, for a
/// batch over the rows of a lane, which row each item is in.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FramedBatched {
    /// The batches, in painting order.
    pub batched: Batched,
    /// Each batch's frame or lane, by its place in `batched.batches`.
    pub frames: Vec<BatchFrame>,
    /// Each batch's rows, by its place: empty for a batch in one frame.
    pub rows: Vec<BatchRows>,
}

/// The rows a batch over a lane's rows draws: `frames[i]` is the lane
/// frame row `i` names, `items[j]` item `j`'s row. Both empty for a batch
/// in one frame.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct BatchRows {
    pub(crate) frames: Vec<usize>,
    pub(crate) items: Vec<u32>,
}

impl BatchRows {
    /// Each item's row, by its place in the batch's run; empty for a
    /// batch in one frame.
    #[must_use]
    pub fn items(&self) -> &[u32] {
        &self.items
    }
}

/// The batches of `steps`' items (each entry's step by `step`) for a
/// target whose drawable area is `view` (device pixels on the page): each
/// run of consecutive steps in one frame, or in the rows of one lane,
/// batched apart.
pub(crate) fn framed_batches<T>(
    steps: &[T],
    step: impl Fn(&T) -> Step,
    sources: Sources<'_>,
    view: Rect,
) -> FramedBatched {
    let resolved = resolved(steps.iter().map(step), sources);
    let mut batches = FramedBatched::default();
    for run in resolved.chunk_by(|a, b| a.group == b.group) {
        batches.add_run(run, sources, view);
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

/// Each of `steps` resolved, each frame's group looked up once.
fn resolved(steps: impl Iterator<Item = Step>, (boxes, frames): Sources<'_>) -> Vec<Resolved> {
    let mut groups: FxHashMap<usize, BatchFrame> = FxHashMap::default();
    let mut group_of = |frame: usize| {
        *groups.entry(frame).or_insert_with(|| {
            frames
                .lane_container_of(frame)
                .map_or_else(|| BatchFrame::frame(frame), BatchFrame::lane)
        })
    };
    let resolve = |step| {
        let frame = frame_items(boxes, step).0;
        let group = group_of(frame);
        Resolved { step, frame, group }
    };
    steps.map(resolve).collect()
}

impl FramedBatched {
    /// The batches of `run`, steps of one group, added.
    fn add_run(&mut self, run: &[Resolved], sources: Sources<'_>, view: Rect) {
        let group = run[0].group;
        match group.0 {
            Framing::Lane(container) => {
                for part in rows_batched(run, (container, sources), view) {
                    self.extend(group, part);
                }
            }
            Framing::Frame(frame) => {
                let (boxes, frames) = sources;
                let items = run.iter().flat_map(|at| frame_items(boxes, at.step).1);
                let part = vapors_scene::batched(items, frames.view_in(frame, view));
                self.extend(group, (part, Vec::new()));
            }
        }
    }

    /// `part`'s batches added, each at `frame`, with its rows (none for a
    /// batch in one frame).
    fn extend(&mut self, frame: BatchFrame, (part, rows): (Batched, Vec<BatchRows>)) {
        let count = part.batches.len();
        self.batched.culled += part.culled;
        self.frames.extend(std::iter::repeat_n(frame, count));
        if rows.is_empty() {
            self.rows
                .extend(std::iter::repeat_n(BatchRows::default(), count));
        } else {
            self.rows.extend(rows);
        }
        self.batched.batches.extend(part.batches);
    }
}

/// How many items `step` paints when every one is culled in `view`
/// (device pixels on the page) and none is a surface (a culled surface
/// keeps its place in a batch); `None` when one shows.
pub(crate) fn hidden_items(step: Step, (boxes, frames): Sources<'_>, view: Rect) -> Option<u64> {
    let (frame, items) = frame_items(boxes, step);
    let view = frames.view_in(frame, view);
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
