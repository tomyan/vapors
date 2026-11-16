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
use crate::layout::Key;

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
    /// frame did: each lane met again, and its rows retested only at its
    /// ends: those from its old first row's start to its new one's and
    /// from its old last row's to its new one's, and those the view's
    /// lines may cut, ink counted, before and now. A row between keeps its
    /// shown steps unread. O(log rows + rows at the ends) a lane, after the
    /// walk that gives the painter each row's frame. Sorted again at the
    /// next draw when a lane gained or lost all its rows in view, lost an
    /// end row, or a row retested holds frames of its own (a scroll box,
    /// a nested lane), whose steps a row's search does not reach.
    pub(crate) fn lanes_moved(&mut self, sources: Sources<'_>, freed: &[(usize, Key)]) {
        if self.unsorted {
            return;
        }
        let Some((retest, now)) = self.ends_moved(sources, freed) else {
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

    /// Panics, naming the row, when a lane's end row at the last search,
    /// freed by this update, is held by its lane again: its slot was taken
    /// by an insert, and would read as the same row moved
    /// (`lane_rows_spanning`), where a freed end row must find no start and
    /// sort the steps again.
    #[cfg(debug_assertions)]
    fn assert_freed_ends_gone(&self, sources: Sources<'_>, freed: &[(usize, Key)]) {
        for (&container, lane) in &self.lanes {
            let ends = lane
                .ends
                .into_iter()
                .flat_map(|(first, last)| [first, last]);
            for end in ends.filter(|&end| freed.iter().any(|&(slot, _)| slot == end)) {
                assert!(
                    !sources.frames.lane_holds(container, end),
                    "row {end}, an end of lane {container} at the last search, was freed by this \
                     update and its lane holds the slot again: an insert took it \
                     (kept_batches_search.rs `ends_moved`); reproduce with `cargo xtask test`"
                );
            }
        }
    }

    /// The rows to retest at each lane's ends, and each lane as met now;
    /// `None` when the steps must be sorted again (`lanes_moved`).
    fn ends_moved(
        &self,
        sources: Sources<'_>,
        freed: &[(usize, Key)],
    ) -> Option<(FxHashSet<usize>, FxHashMap<usize, LaneSeen>)> {
        #[cfg(debug_assertions)]
        self.assert_freed_ends_gone(sources, freed);
        #[cfg(not(debug_assertions))]
        let _ = freed; // only the debug build's check reads it
        let now = lanes_seen(&frames_to_search(self.view, sources), self.view, sources);
        let mut retest = FxHashSet::default();
        let containers = self
            .lanes
            .keys()
            .chain(now.keys().filter(|c| !self.lanes.contains_key(c)));
        for container in containers {
            let (was, seen) = (self.lanes.get(container), now.get(container));
            retest.extend(lane_ends(*container, (was, seen), sources)?);
        }
        let holding = retest.iter().any(|&row| sources.frames.holds_frames(row));
        (!holding).then_some((retest, now))
    }
}

/// The rows of lane `container` to retest, met as `was` at the last
/// search and as `seen` now (`None`: not met); `None` when it gained or
/// lost all its rows in view, or an end row is gone.
fn lane_ends(
    container: usize,
    (was, seen): (Option<&LaneSeen>, Option<&LaneSeen>),
    sources: Sources<'_>,
) -> Option<Vec<usize>> {
    let ends = |lane: Option<&LaneSeen>| lane.and_then(|lane| lane.ends);
    let (Some(was_ends), Some(seen_ends)) = (ends(was), ends(seen)) else {
        return (ends(was).is_none() && ends(seen).is_none()).then(Vec::new);
    };
    let spanning = |a, b| sources.frames.lane_rows_spanning(container, (a, b));
    let mut rows = spanning(was_ends.0, seen_ends.0)?;
    rows.extend(spanning(was_ends.1, seen_ends.1)?);
    let edges = was
        .into_iter()
        .chain(seen)
        .flat_map(|lane| lane.edges.iter().copied());
    rows.extend(edges);
    Some(rows)
}

/// Each framed lane in `frames` (those searched) as it meets `view`,
/// those with no row in view too.
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

#[cfg(test)]
#[path = "kept_batches_search_cases_tests.rs"]
mod cases_tests;
