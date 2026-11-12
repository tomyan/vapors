//! What the last update did, counted: the cache misses the incremental
//! tests hold (restyled, laid out, text built) and the structural work
//! of each phase.

use crate::document::Document;
use crate::layout::LayoutWork;
use crate::painted::Painted;

/// Structural counts of the work the last [`Document::update`] did, for
/// diagnostics and tests. They count units of work, not time.
///
/// Every count is reset to 0 when an update starts, and each phase sets
/// its own as it runs, so a refused update reports the phases it ran
/// and 0 for those after the refusal. Before the first update, all are
/// 0.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct UpdateCounts {
    /// Elements the style phase checked again after restyling: those
    /// restyled, marked or added since the last update; every element
    /// on the first update.
    pub elements_visited: usize,
    /// Elements the style phase restyled; 0 when the restyle failed (the
    /// partial count of a failed restyle is not kept).
    pub elements_restyled: usize,
    /// Layout boxes built from the styled tree: one per element that
    /// is neither `display: none` nor inline (the root, block-level and
    /// replaced elements, and an `svg` as one box with none for its
    /// children), and one per text run. A `display: none` element's
    /// subtree and inline elements, whose text is part of a run, have
    /// none.
    pub boxes_built: usize,
    /// Times Taffy asked a box for its layout, a cache hit included. A
    /// box can be asked more than once in an update (measured, then
    /// laid out), and a box under a cache hit is not asked at all.
    pub layout_calls: usize,
    /// Times Taffy laid a box out rather than reading its cache.
    pub boxes_laid_out: usize,
    /// Rebuilt boxes whose subtree changed and that layout laid out in
    /// isolation (`layout/isolated.rs`): asked what their parent asked
    /// the box they replaced, they answered the same, so their
    /// ancestors kept their cached layouts.
    pub owners_isolated: usize,
    /// Rebuilt boxes whose subtree changed and that answered otherwise
    /// (a size, margin or baseline changed, or an out-of-flow box
    /// bubbles up): their ancestors were laid out again.
    pub owners_resized: usize,
    /// Rebuilt boxes whose subtree changed with nothing to compare
    /// against (no box held the slot, or it was never asked): their
    /// ancestors were laid out again.
    pub owners_uncompared: usize,
    /// Boxes whose rect layout worked out again (IU3 3b): those Taffy
    /// laid out or gave a new layout, those built again or scrolled,
    /// the boxes above them, and the boxes placed in a box whose
    /// content moved. Every box when nothing is kept.
    pub boxes_placed: usize,
    /// Boxes not built again whose rect or padding changed since the
    /// last paint; 0 when nothing is kept.
    pub boxes_moved: usize,
    /// Slots whose stacking (`stacking.rs`) was made from their style:
    /// those built again (IU3 kept stackings); every held slot when
    /// nothing is kept.
    pub stackings_computed: usize,
    /// Boxes paint drew: one per box in painting order, outlines and
    /// scroll thumbs not included. A box built again, moved, or cut by
    /// another clip is drawn; the others' kept paint stays in place
    /// (`kept_box.rs`), and none is drawn when the update showed the
    /// last update's paint (`kept_paint.rs`).
    pub boxes_painted: usize,
    /// Boxes whose kept paint paint read and kept rather than drew:
    /// those moved or whose clip was worked out again, but whose paint
    /// was made from the same rect, clip and padding (IU3 3c-3d2). The
    /// kept paint of every other box is neither read nor copied.
    pub parts_visited_kept: usize,
    /// Scene items (paint records) paint built: those of the boxes it
    /// drew, their outlines, and every scroll thumb; kept items are
    /// not counted.
    pub scene_items_built: usize,
    /// Elements whose entries in the border and padding box maps paint
    /// replaced or removed: those built again, moved or gone since the
    /// last paint (IU3 3c-3b); every element when paint built the maps
    /// afresh, and none when the update showed the last update's paint.
    pub map_entries_patched: usize,
    /// Times paint order (the paint steps, with each thumb's place)
    /// was walked over the whole tree: 1 when the tree's shape changed
    /// since the last paint (a box added, removed or moved among its
    /// siblings, or its stacking changed), else 0 (IU3 3c-3c).
    pub order_walks: usize,
    /// Times the scroll chains were walked over the whole tree: 1 when
    /// the shape changed, else 0.
    pub chain_walks: usize,
    /// Boxes whose clip paint worked out again: those built again or
    /// moved, and below them those whose clip comes from a box whose
    /// clip changed; every slot when the shape changed.
    pub clips_redone: usize,
}

impl UpdateCounts {
    /// Notes what layout did.
    pub(crate) fn count_layout(&mut self, work: &LayoutWork) {
        (self.boxes_laid_out, self.layout_calls) = (work.laid_out, work.calls);
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
        self.parts_visited_kept = painted.parts_visited_kept;
        self.scene_items_built = painted.items_built;
        self.map_entries_patched = painted.maps_patched;
        let walks = painted.order.walks;
        (self.order_walks, self.chain_walks, self.clips_redone) =
            (walks.order, walks.chains, walks.clips);
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

    /// How many Parley layouts the last update built rather than kept
    /// from the one before: what the incremental tests count, and what
    /// the `stress` demo shows.
    #[must_use]
    pub fn text_layouts_built(&self) -> usize {
        self.texts.built()
    }
}
