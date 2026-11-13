//! Batching in scroll frames (IU4 4c, `docs/plans/iu4.md` section 3,
//! "Batching" and "Culling"): each item stays in its part's frame's
//! coordinates under its clip inside the frame, and is culled against
//! the view as it falls in that frame, cut to the frame's clip.
//! Consecutive items share a batch only within one frame, so each batch
//! names one frame, whose shift and clip the painter applies at draw.

use vapors_scene::{Batched, Item, Rect};

use crate::frames::Frames;
use crate::frames_draw::BatchFrame;
use crate::kept_box::KeptBox;
use crate::paint_order::Step;
use crate::scene_view::frame_items;

/// The batches of `steps`' items (each entry's step by `step`) for a
/// target whose drawable area is `view` (device pixels on the page),
/// with each batch's frame: each run of consecutive steps in one frame
/// batched alone, in that frame's view.
pub(crate) fn framed_batches<T>(
    steps: &[T],
    step: impl Fn(&T) -> Step,
    (boxes, frames): (&[Option<KeptBox>], &Frames),
    view: Rect,
) -> (Batched, Vec<BatchFrame>) {
    let frame_of = |entry: &T| frame_items(boxes, step(entry)).0;
    let (mut batched, mut of) = (Batched::default(), Vec::new());
    for run in steps.chunk_by(|a, b| frame_of(a) == frame_of(b)) {
        let frame = frame_of(&run[0]);
        let items = run
            .iter()
            .flat_map(|entry| frame_items(boxes, step(entry)).1);
        let part = vapors_scene::batched(items, frames.view_in(frame, view));
        batched.culled += part.culled;
        of.extend(std::iter::repeat_n(BatchFrame(frame), part.batches.len()));
        batched.batches.extend(part.batches);
    }
    (batched, of)
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
