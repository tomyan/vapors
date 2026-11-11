//! What the last update did, counted: the cache misses the incremental
//! tests hold (restyled, laid out, text built) and the structural work
//! of each phase.

use crate::document::Document;
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
    /// Boxes paint drew: one per box in painting order, outlines and
    /// scroll thumbs not included.
    pub boxes_painted: usize,
    /// Scene items (paint records) paint built. Paint builds the scene
    /// from empty on every update, so this is every item in it.
    pub scene_items_built: usize,
}

impl UpdateCounts {
    /// Notes what paint drew into `painted`.
    pub(crate) fn note_paint(&mut self, painted: &Painted) {
        self.boxes_painted = painted.boxes_drawn;
        self.scene_items_built = painted.scene.items().len();
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
