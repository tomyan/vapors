//! What the kept steps remember of each framed lane in the view, and
//! what changed at its ends after its rows moved
//! (`kept_batches_search.rs`; `docs/plans/splice-layout.md` section 11.2,
//! 4f-2c): the rows that left or entered, and those the view cuts, are
//! found by the starts between the old and new end rows, so no row between
//! them is read.

use vapors_scene::Rect;

use super::super::Frames;

/// A framed lane as a search of the view found it: its first and last
/// rows meeting the view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LaneSeen {
    pub(crate) first: usize,
    pub(crate) last: usize,
}

impl Frames {
    /// The framed lane containers whose content is in frame `frame`.
    pub(crate) fn lanes_in(&self, frame: usize) -> &[usize] {
        self.lanes.by_parent.get(&frame).map_or(&[], Vec::as_slice)
    }

    /// Framed lane `container` as it meets `view`, device pixels on the
    /// page; `None` when no row of it meets the view. The walk over the
    /// rows meeting the view memoises each row's frame (the painter moves
    /// every one); nothing else is read of a row between the edges.
    pub(crate) fn lane_seen(&self, container: usize, view: Rect) -> Option<LaneSeen> {
        let base = *self.lanes.bases.get(&container)?;
        let in_frame = self.view_in(base.parent, view);
        let rows = self.lane_rows_meeting(container, in_frame);
        let (&first, &last) = (rows.first()?, rows.last()?);
        Some(LaneSeen { first, last })
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
