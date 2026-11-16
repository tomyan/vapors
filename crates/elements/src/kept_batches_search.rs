//! Finding the steps that show in a view (`kept_batches.rs`): each frame
//! that can show searched in the extents index, a framed lane's rows
//! found by one walk of its tree (`Frames::meeting`); and, after rows of
//! framed lanes moved, only the rows that were or are at the view's edges
//! searched again (`docs/plans/splice-layout.md` section 11.2, 4f-2b).

use rustc_hash::{FxHashMap, FxHashSet};
use vapors_scene::Rect;

use super::{KeptBatches, Shown, Sources};
use crate::framed_batches::hidden_in;

/// The frames whose steps can show in `view`: those whose container's
/// clip meets it (`frames_nested.rs`), and those holding a step that
/// spans every view, which a culled surface's batch needs; each once.
fn frames_to_search(view: Rect, sources: Sources<'_>) -> Vec<usize> {
    let mut frames = sources.frames.meeting(view);
    frames.extend(sources.extents.spanning_frames());
    frames.sort_unstable();
    frames.dedup();
    frames
}

impl KeptBatches {
    /// The steps of `sources` that show in `view`: in each frame that
    /// can show in it, the extents index's candidates for the view as it
    /// falls in the frame, tested item by item.
    pub(crate) fn of(view: Rect, sources: Sources<'_>) -> Self {
        #[cfg(test)]
        super::BUILDS.with(|builds| builds.set(builds.get() + 1));
        let mut shown = Vec::new();
        let mut lane_rows = FxHashMap::default();
        // Only the frames whose container's clip meets the view are
        // searched (`frames_nested.rs`): O(shown frames · log steps).
        for frame in frames_to_search(view, sources) {
            let in_frame = sources.frames.view_in(frame, view);
            if let Some(inside) = lane_row_inside(frame, in_frame, sources) {
                lane_rows.insert(frame, inside);
            }
            shown.extend(showing(frame, in_frame, sources));
        }
        shown.sort_unstable_by_key(|shown| shown.label);
        let shown_items = shown.iter().map(|shown| shown.items).sum();
        Self {
            view,
            shown,
            shown_items,
            shown_culled: 0,
            batched: None,
            stale: true,
            unsorted: false,
            lane_rows,
        }
    }

    /// After rows of framed lanes moved (started elsewhere) and no other
    /// frame did: the lanes searched again, one walk each. A row whose
    /// items lay wholly inside the view at the last search and do now
    /// keeps its shown steps; a row that left gives up its steps; any
    /// other row found now (entered, or at an edge either time) is tested
    /// step by step. O(log rows + rows at the edges), not O(rows in view).
    pub(crate) fn lanes_moved(&mut self, sources: Sources<'_>) {
        if self.unsorted {
            return;
        }
        let (mut now, mut retest) = (FxHashMap::default(), Vec::new());
        for frame in frames_to_search(self.view, sources) {
            let in_frame = sources.frames.view_in(frame, self.view);
            let Some(inside) = lane_row_inside(frame, in_frame, sources) else {
                continue;
            };
            now.insert(frame, inside);
            if !(inside && self.lane_rows.get(&frame) == Some(&true)) {
                retest.push((frame, in_frame));
            }
        }
        let left = self.lane_rows.keys().filter(|row| !now.contains_key(row));
        let dropped: FxHashSet<usize> = retest
            .iter()
            .map(|&(row, _)| row)
            .chain(left.copied())
            .collect();
        self.retain_shown(|shown| !dropped.contains(&shown.frame));
        for (frame, in_frame) in retest {
            showing(frame, in_frame, sources).for_each(|shown| self.insert_shown(shown));
        }
        self.lane_rows = now;
    }
}

/// The steps of `frame`'s index with an item showing in `in_frame`, the
/// view in its coordinates.
fn showing(frame: usize, in_frame: Rect, sources: Sources<'_>) -> impl Iterator<Item = Shown> + '_ {
    let candidates = sources.extents.candidates(frame, in_frame);
    let shows = move |entry: &&crate::frame_extents::Entry| {
        hidden_in(entry.step, (sources.boxes, frame), in_frame).is_none()
    };
    candidates.filter(shows).map(move |entry| Shown {
        label: entry.label,
        step: entry.step,
        items: entry.items,
        frame,
    })
}

/// Whether the items of `frame`, a framed lane's row, lie wholly inside
/// `in_frame` along the lane (rows move only down a lane, so across it
/// nothing changes); `None` when `frame` is no lane row.
fn lane_row_inside(frame: usize, in_frame: Rect, sources: Sources<'_>) -> Option<bool> {
    sources.frames.lane_container_of(frame)?;
    let span = sources.extents.frame_span(frame);
    Some(span.is_none_or(|[top, bottom]| in_frame.top <= top && bottom <= in_frame.bottom))
}

#[cfg(test)]
#[path = "kept_batches_search_tests.rs"]
mod tests;
