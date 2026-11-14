//! What one layout wrote, by slot (IU3 3b, `docs/plans/iu3.md` "3b:
//! kept placement"): Taffy lays a box out only through
//! `compute_child_layout`, writes its layout only through
//! `set_unrounded_layout`, and its hoisted list only in
//! `compute_oof_layout`, so a box in none of these lists holds the
//! layout and hoisted list it held after the last layout.

/// The slots one layout laid out or wrote.
#[derive(Debug, Default)]
pub(crate) struct Written {
    /// Slots laid out rather than read from their cache, in the order
    /// Taffy laid them out; a slot can appear more than once.
    pub(crate) laid_out: Vec<usize>,
    /// Slots whose layout was set to one other than they held.
    pub(crate) changed: Vec<usize>,
    /// Slots whose hoisted list was written.
    pub(crate) hoisting: Vec<usize>,
    /// Lane children whose start along the axis was written to another
    /// than they held (`lane_layout.rs`): a framed child's rect leaves
    /// its start out, so paint and the a11y take learn of its new place
    /// from this list.
    pub(crate) started: Vec<usize>,
}
