//! Batching in scroll frames (IU4 4c, `docs/plans/iu4.md` section 3,
//! "Batching" and "Culling"): each item stays in its part's frame's
//! coordinates under its clip inside the frame, and is culled against
//! the view as it falls in that frame, cut to the frame's clip.
//! Consecutive items share a batch within one frame, or across the rows
//! of one framed lane (`docs/plans/splice-layout.md` section 11.2, 4f):
//! such a batch names the lane, each item its row, whose offset the
//! painter adds per instance. A lane's run is split at
//! [`ROWS_PER_BATCH`] rows.

use rustc_hash::{FxHashMap, FxHashSet};

use vapors_scene::{Batched, Item, ROWS_PER_BATCH, Rect};

use crate::frames::Frames;
use crate::frames_draw::{BatchFrame, Framing};
use crate::kept_box::KeptBox;
use crate::paint_order::Step;
use crate::scene_view::frame_items;

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
    (boxes, frames): (&[Option<KeptBox>], &Frames),
    view: Rect,
) -> FramedBatched {
    let frame_of = |entry: &T| frame_items(boxes, step(entry)).0;
    let group = |entry: &T| {
        let frame = frame_of(entry);
        frames
            .lane_container_of(frame)
            .map_or_else(|| BatchFrame::frame(frame), BatchFrame::lane)
    };
    let mut batches = FramedBatched::default();
    for run in steps.chunk_by(|a, b| group(a) == group(b)) {
        let at = group(&run[0]);
        let rowed = run.iter().map(|entry| (step(entry), frame_of(entry)));
        match at.0 {
            Framing::Lane(_) => {
                for part in rows_batched(rowed, (boxes, frames), view) {
                    batches.extend(at, part);
                }
            }
            Framing::Frame(frame) => {
                let items = rowed.flat_map(|(step, _)| frame_items(boxes, step).1);
                let part = vapors_scene::batched(items, frames.view_in(frame, view));
                batches.extend(at, (part, Vec::new()));
            }
        }
    }
    batches
}

impl FramedBatched {
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

/// The batches of `steps`' items, each step with its row's frame and
/// each item culled in its row's view, and each batch's rows: a part for
/// each [`ROWS_PER_BATCH`] rows with an item batched, so the split falls
/// where the items shown fall, whichever steps are given (every drawn
/// step, or those the extents index finds showing).
fn rows_batched(
    steps: impl Iterator<Item = (Step, usize)>,
    (boxes, frames): (&[Option<KeptBox>], &Frames),
    view: Rect,
) -> Vec<(Batched, Vec<BatchRows>)> {
    let mut parts = Vec::new();
    let (mut part, mut item_rows) = (Batched::default(), Vec::new());
    let mut rows: FxHashSet<usize> = FxHashSet::default();
    for (step, row) in steps {
        let in_row = frames.view_in(row, view);
        for (item, clip) in frame_items(boxes, step).1 {
            let shows =
                !vapors_scene::culled(&item, clip, in_row) || matches!(item, Item::Surface { .. });
            if shows && !rows.contains(&row) && rows.len() == ROWS_PER_BATCH {
                let full = std::mem::take(&mut part);
                parts.push((full, std::mem::take(&mut item_rows)));
                rows.clear();
            }
            if part.add(&item, clip, in_row) {
                item_rows.push(row);
                rows.insert(row);
            }
        }
    }
    parts.push((part, item_rows));
    parts
        .into_iter()
        .map(|(part, item_rows)| {
            let rows = rows_of(&part, &item_rows);
            (part, rows)
        })
        .collect()
}

/// Each of `part`'s batches' rows, from each batched item's row frame in
/// order: a batch's rows numbered as they first appear in it.
fn rows_of(part: &Batched, item_rows: &[usize]) -> Vec<BatchRows> {
    let mut items = item_rows.iter();
    let mut batch_rows = |len: usize| {
        let mut index: FxHashMap<usize, u32> = FxHashMap::default();
        let mut rows = BatchRows::default();
        for &row in items.by_ref().take(len) {
            let next = u32::try_from(rows.frames.len()).expect("under 2^32 rows");
            let at = *index.entry(row).or_insert(next);
            if at == next {
                rows.frames.push(row);
            }
            rows.items.push(at);
        }
        rows
    };
    part.batches
        .iter()
        .map(|batch| batch_rows(batch.run.len()))
        .collect()
}

/// How many items `step` paints when every one is culled in `view`
/// (device pixels on the page) and none is a surface (a culled surface
/// keeps its place in a batch); `None` when one shows.
pub(crate) fn hidden_items(
    step: Step,
    (boxes, frames): (&[Option<KeptBox>], &Frames),
    view: Rect,
) -> Option<u64> {
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
