//! What the last update did, counted: the cache misses the incremental
//! tests hold (restyled, laid out, text built) and the structural work
//! of each phase.

use crate::splice_decline::SpliceDecline;

/// Structural counts of the work the last [`Document::update`] did, for
/// diagnostics and tests: units of work, not time. Every count resets
/// to 0 when an update starts and each phase sets its own as it runs,
/// so a refused update reports the phases it ran and 0 for those after
/// the refusal; before the first update, all are 0.
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
    /// Restyled elements that took a computed style from the share
    /// table: keyed (`Dom::create_keyed_element`), their key shareable,
    /// and an element with the same share key restyled before them
    /// (`restyle_share.rs`). This and the next three are 0 when the
    /// restyle failed, as `elements_restyled` is.
    pub styles_shared: usize,
    /// Restyled keyed, shareable elements whose style was computed and
    /// put in the share table: one per distinct share key restyled.
    pub styles_computed: usize,
    /// Restyled elements created without a key (hand-built pages, the
    /// root and body), styled each for itself as before sharing.
    pub styles_unkeyed: usize,
    /// Restyled keyed elements whose key cannot share: a rule any of
    /// its elements can match reads siblings or descendants, or sits
    /// under `@container`.
    pub styles_unshareable: usize,
    /// Computed style allocations alive after the update: the share
    /// table's entries and every element styled for itself.
    pub styles_live: usize,
    /// Layout boxes built from the styled tree: one per element that
    /// is neither `display: none` nor inline (the root, block-level and
    /// replaced elements, and an `svg` as one box with none for its
    /// children), and one per text run. A `display: none` element's
    /// subtree and inline elements, whose text is part of a run, have
    /// none.
    pub boxes_built: usize,
    /// Elements whose box children an insert or remove edited in place
    /// (`layout/child_edits.rs`): the inserted children built and put
    /// in after their previous sibling's box, the removed taken out,
    /// the other children not walked.
    pub children_edited: usize,
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
    /// bubbles up), or whose style or natural size changed: their
    /// ancestors were laid out again.
    pub owners_resized: usize,
    /// Rebuilt boxes whose subtree changed with nothing to compare
    /// against: no box held the slot, it was never asked, the old
    /// cache lost a layout's input, its size answer bubbled an
    /// out-of-flow box up or was disowned, it was asked more ways
    /// than it records, it is absolute or fixed, it is restyled with
    /// a calc, it is the root or it is a text box (see
    /// `layout/isolated.rs`). Their ancestors were laid out again.
    pub owners_uncompared: usize,
    /// Recorded size answers disowned this update: a box answered a
    /// size query otherwise than its record, which cannot happen
    /// unless something under it changed without its cache being
    /// cleared; the record is dropped so the box is not compared
    /// (`layout/tree_cache.rs`). 0 on every update today.
    pub size_answers_disowned: usize,
    /// Boxes whose rect layout worked out again (IU3 3b): those Taffy
    /// laid out or gave a new layout, those built again or scrolled,
    /// the boxes above them, and the boxes placed in a box whose
    /// content moved. Every box when nothing is kept.
    pub boxes_placed: usize,
    /// Boxes not built again whose rect or padding changed since the
    /// last paint; 0 when nothing is kept.
    pub boxes_moved: usize,
    /// In-flow children chained to the slot they are placed in before
    /// the placement walk (M10b, `placement.rs`): those of each slot
    /// built again or laid out, each such slot once. 0 when nothing is
    /// kept, as the walk chains every slot then.
    pub slots_chained: usize,
    /// Boxes marked kept for the next update (`KeptBoxes::keep`): those
    /// built this update; every other box was kept already.
    pub boxes_kept: usize,
    /// Slots whose stacking (`stacking.rs`) was made from their style:
    /// those built again (IU3 kept stackings); every held slot when
    /// nothing is kept.
    pub stackings_computed: usize,
    /// Boxes paint drew: one per box in painting order, outlines and
    /// scroll thumbs not included. A box built again, or cut by
    /// another clip, is drawn, as is a moved box whose kept paint
    /// cannot be shifted exactly; the others' kept paint stays in place
    /// (`kept_box.rs`), and none is drawn when the update showed the
    /// last update's paint (`kept_paint.rs`).
    pub boxes_painted: usize,
    /// Boxes that moved or were cut by another clip whose kept paint
    /// was shifted in place of being drawn (IU3 3c-3e,
    /// `kept_shift.rs`): the box kept its size and padding and lies on
    /// whole device pixels before and after, its edges snapping as
    /// they did, moved.
    pub boxes_shifted: usize,
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
    /// Times the kept paint order, clip tree, clips and chains were
    /// spliced for inserted and removed in-flow boxes in place of being
    /// walked (`order_splice.rs`, `docs/plans/splice.md` (c)): 1 or 0.
    pub order_splices: usize,
    /// Times a shape change could not be spliced and the whole tree was
    /// walked instead: a changed slot's children reordered, a box with
    /// a stacking other than the default inserted or removed, or a
    /// rebuilt box's stacking changed; 1 or 0.
    pub splices_declined: usize,
    /// The condition that declined the splice when `splices_declined`
    /// is 1, else `None`.
    pub splice_decline: Option<SpliceDecline>,
    /// Boxes whose clip paint worked out again: those built again or
    /// moved, and below them those whose clip comes from a box whose
    /// clip changed; every slot when the shape changed.
    pub clips_redone: usize,
    /// Times the update hit tested the pointer's position after paint
    /// (`events_hit.rs`): 1 when a hit paint removed or added holds the
    /// pointer or is the hovered element's, or the hits were gathered
    /// afresh or not gathered since the last update; else 0, and the
    /// hover stands, as it does while an element holds the mouse
    /// capture (M11).
    pub hovers_tested: usize,
}
