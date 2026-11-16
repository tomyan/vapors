//! Finding the steps that show in a view (`kept_batches.rs`): each frame
//! that can show searched in the extents index, a framed lane's rows
//! found by one walk of its tree (`Frames::meeting`); and, after rows of
//! framed lanes moved, only the rows that were or are at the view's edges
//! searched again (`docs/plans/splice-layout.md` section 11.2, 4f-2b).

use rustc_hash::{FxHashMap, FxHashSet};
use vapors_scene::Rect;

use super::{KeptBatches, Shown, Sources};
use crate::framed_batches::hidden_in;
use crate::frames::LaneSeen;

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
    /// falls in the frame, tested item by item; and each framed lane as
    /// the search met it.
    pub(crate) fn of(view: Rect, sources: Sources<'_>) -> Self {
        #[cfg(test)]
        super::BUILDS.with(|builds| builds.set(builds.get() + 1));
        let mut shown = Vec::new();
        // Only the frames whose container's clip meets the view are
        // searched (`frames_nested.rs`): O(shown frames · log steps).
        let frames = frames_to_search(view, sources);
        for &frame in &frames {
            let in_frame = sources.frames.view_in(frame, view);
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
            lanes: lanes_seen(&frames, view, sources),
        }
    }

    /// After rows of framed lanes moved (started elsewhere) and no other
    /// frame did: each lane met again, its rows retested only at its ends,
    /// those from its old first row's start to its new one's and from its
    /// old last row's to its new one's, each end row included (a row the
    /// view cuts is an end row); a row between keeps its shown steps
    /// unread. O(log rows + rows at the ends) a
    /// lane, after the walk that gives the painter each row's frame.
    /// Sorted again at the next draw when a lane left the view or lost an
    /// end row.
    pub(crate) fn lanes_moved(&mut self, sources: Sources<'_>) {
        if self.unsorted {
            return;
        }
        let Some((retest, now)) = self.ends_moved(sources) else {
            self.unsort();
            return;
        };
        self.retain_shown(|shown| !retest.contains(&shown.frame));
        for &row in &retest {
            if sources.frames.lane_container_of(row).is_some() {
                let in_row = sources.frames.view_in(row, self.view);
                showing(row, in_row, sources).for_each(|shown| self.insert_shown(shown));
            }
        }
        self.lanes = now;
    }

    /// The rows to retest at each lane's ends, and each lane as met now;
    /// `None` when a lane left the view or an end row is gone.
    fn ends_moved(
        &self,
        sources: Sources<'_>,
    ) -> Option<(FxHashSet<usize>, FxHashMap<usize, LaneSeen>)> {
        let (mut retest, mut now) = (FxHashSet::default(), FxHashMap::default());
        for (&container, was) in &self.lanes {
            let seen = sources.frames.lane_seen(container, self.view)?;
            let spanning = |a, b| sources.frames.lane_rows_spanning(container, (a, b));
            retest.extend(spanning(was.first, seen.first)?);
            retest.extend(spanning(was.last, seen.last)?);
            now.insert(container, seen);
        }
        Some((retest, now))
    }
}

/// Each framed lane in `frames` (those searched) as it meets `view`.
fn lanes_seen(frames: &[usize], view: Rect, sources: Sources<'_>) -> FxHashMap<usize, LaneSeen> {
    let containers = frames
        .iter()
        .flat_map(|&frame| sources.frames.lanes_in(frame));
    let seen = |&container: &usize| Some((container, sources.frames.lane_seen(container, view)?));
    containers.filter_map(seen).collect()
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

#[cfg(test)]
#[path = "kept_batches_search_tests.rs"]
mod tests;
