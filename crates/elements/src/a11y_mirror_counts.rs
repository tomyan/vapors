//! What the last a11y take did, counted (R6): units of work, not time,
//! as `UpdateCounts` counts an update. The scaling tests hold them
//! across page sizes: a take's work follows what was touched, never the
//! page, except where `docs/plans/r6.md` names the cost.

use crate::document::Document;

/// Structural counts of the work the last [`Document::take_ax_update`]
/// did. Every count is reset to 0 when a take starts, so a take refused
/// reports the work up to the refusal, and a take that found nothing
/// reports 0 for everything but the index.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct AxTakeCounts {
    /// Nodes read again and compared with their record: every node a
    /// touch reached, each once; every node for a take of all.
    pub nodes_read: usize,
    /// Index entries read: the touched nodes when the index was
    /// patched, every element when it was scanned.
    pub index_entries: usize,
    /// 1 when the index was rebuilt by a scan of the document (the
    /// first take; an element inserted or removed; an id carried by
    /// more than one element), else 0.
    pub index_scans: usize,
    /// Steps the placement ordering took: each placement sorted into
    /// ready or waiting, released by the one it waited on, and placed;
    /// at most three times the placements.
    pub placements_considered: usize,
}

impl Document {
    /// The structural counts of the last take: see [`AxTakeCounts`].
    #[must_use]
    pub fn ax_take_counts(&self) -> AxTakeCounts {
        self.ax.counts
    }
}
