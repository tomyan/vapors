//! How the counts are filled by the phases (`update_counts.rs`) and
//! read from the document.

use crate::document::Document;
use crate::layout::LayoutWork;
use crate::painted::Painted;
use crate::update_counts::UpdateCounts;

impl UpdateCounts {
    /// The most entries one of paint's ordered indexes moves per patch
    /// besides block pointers: `blocks.rs` `CAP`, the block size.
    pub const INDEX_BLOCK: usize = crate::blocks::CAP;

    /// Notes what layout did.
    pub(crate) fn count_layout(&mut self, work: &LayoutWork) {
        (self.boxes_laid_out, self.layout_calls) = (work.laid_out, work.calls);
        self.size_answers_disowned = work.disowned;
        let settled = work.settled;
        (
            self.owners_isolated,
            self.owners_resized,
            self.owners_uncompared,
        ) = (settled.isolated, settled.resized, settled.uncompared);
    }

    /// Notes what paint drew into `painted`.
    pub(crate) fn note_paint(&mut self, painted: &Painted) {
        self.boxes_painted = painted.boxes_drawn;
        self.boxes_shifted = painted.boxes_shifted;
        self.parts_visited_kept = painted.parts_visited_kept;
        self.scene_items_built = painted.items_built;
        self.map_entries_patched = painted.maps_patched;
        let indexes = painted.index_counts;
        (self.index_entries_moved, self.index_rebuilds) = (indexes.moved, indexes.rebuilds);
        self.index_blocks = indexes.blocks;
        let walks = painted.order.walks;
        (self.order_walks, self.chain_walks, self.clips_redone) =
            (walks.order, walks.chains, walks.clips);
        self.order_splices = walks.splices;
        self.splices_declined = usize::from(walks.declined.is_some());
        self.splice_decline = walks.declined;
    }
}

impl Document {
    /// The structural counts of the last update: see [`UpdateCounts`].
    #[must_use]
    pub fn update_counts(&self) -> UpdateCounts {
        self.counts
    }

    /// How many elements the last update restyled: what the incremental
    /// tests count, and what the `stress` demo shows.
    #[must_use]
    pub fn elements_restyled(&self) -> usize {
        self.counts.elements_restyled
    }

    /// How many times the last update's layout laid a box out rather
    /// than reading its cache: what the incremental tests count.
    #[must_use]
    pub fn boxes_laid_out(&self) -> usize {
        self.counts.boxes_laid_out
    }
}
