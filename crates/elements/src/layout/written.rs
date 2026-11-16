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
    /// Each lane container whose children's starts may have changed,
    /// with the slot of the first such child: it and every later child
    /// (`Lanes::children_from`). A lane child's layout leaves its start
    /// out (`lane_layout.rs`), so placement of a lane that is not
    /// framed, paint and the a11y take learn of new starts from this
    /// list: one entry per lane, whatever the children.
    pub(crate) started: Vec<(usize, usize)>,
}
