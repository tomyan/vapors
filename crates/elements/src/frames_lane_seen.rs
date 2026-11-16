//! What the kept steps remember of each framed lane in the view, and
//! what changed at its ends after its rows moved
//! (`kept_batches_search.rs`; `docs/plans/splice-layout.md` section 11.2,
//! 4f-2c): the rows that left or entered, and those the view cuts, are
//! found by the starts between the old and new end rows, so no row between
//! them is read.

use vapors_scene::Rect;

use super::super::Frames;

/// A framed lane as a search of the view found it: its first and last
/// rows meeting the view, if any did, and the rows the view's lines may
/// cut, ink counted (`Frames::lane_edges`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LaneSeen {
    pub(crate) ends: Option<(usize, usize)>,
    pub(crate) edges: Vec<usize>,
}

impl Frames {
    /// The framed lane containers whose content is in frame `frame`.
    pub(crate) fn lanes_in(&self, frame: usize) -> &[usize] {
        self.lanes.by_parent.get(&frame).map_or(&[], Vec::as_slice)
    }

    /// Framed lane `container` as it meets `view`, device pixels on the
    /// page; `None` when it is no framed lane. The walk over the rows
    /// meeting the view memoises each row's frame (the painter moves every
    /// one); nothing else is read of a row between the edges.
    pub(crate) fn lane_seen(&self, container: usize, view: Rect) -> Option<LaneSeen> {
        let base = *self.lanes.bases.get(&container)?;
        let in_frame = self.view_in(base.parent, view);
        let rows = self.lane_rows_meeting(container, in_frame);
        let ends = rows
            .first()
            .zip(rows.last())
            .map(|(&first, &last)| (first, last));
        let edges = self.lane_edges(container, base, in_frame);
        Some(LaneSeen { ends, edges })
    }

    /// Whether framed lane `container` holds a row in slot `row`.
    #[cfg(debug_assertions)]
    pub(crate) fn lane_holds(&self, container: usize, row: usize) -> bool {
        let lanes = self.lanes.shared.as_ref().map(|lanes| lanes.borrow());
        lanes.is_some_and(|lanes| lanes.holds(container, row))
    }

    /// Whether frame `frame` holds frames of its own: a scroll container
    /// in it (`Frames::meeting`'s index), or a framed lane whose content
    /// is in it.
    pub(crate) fn holds_frames(&self, frame: usize) -> bool {
        let scrollers = self
            .index
            .get(&frame)
            .is_some_and(|index| !index.is_empty());
        scrollers || !self.lanes_in(frame).is_empty()
    }

    /// The rows of framed lane `container` from the start of row `a` to
    /// that of row `b`, either way round, as they start now; `None` when
    /// the lane no longer holds either.
    pub(crate) fn lane_rows_spanning(
        &self,
        container: usize,
        (a, b): (usize, usize),
    ) -> Option<Vec<usize>> {
        let lanes = self.lanes.shared.as_ref()?.borrow();
        let (start_a, start_b) = (
            lanes.frame_offset(container, a)?,
            lanes.frame_offset(container, b)?,
        );
        let (from, to) = (start_a.min(start_b), start_a.max(start_b));
        let rows = lanes.children_between(container, from, to);
        Some(rows.into_iter().map(|(slot, _)| slot).collect())
    }
}
