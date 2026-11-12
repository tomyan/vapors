//! Why the order splice declined a shape change (`order_splice.rs`),
//! so a test can pin the condition it names: every decline walks the
//! whole tree, which is the same whichever condition fired.

/// The condition that made the kept paint order be walked afresh in
/// place of spliced (`UpdateCounts::splice_decline`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SpliceDecline {
    /// A kept box's stacking changed (it gained or lost a z-index,
    /// position or scroll).
    Restacked,
    /// A freed box had a stacking other than the default.
    FreedStacked,
    /// The change did not reshape exactly one slot's children.
    NotOneOwner,
    /// A box left the slot's children and still holds a box (it moved
    /// to another parent).
    Moved,
    /// The kept children are not in their old order.
    Reordered,
    /// An inserted box has a stacking other than the default.
    InsertedStacked,
    /// An inserted box already has a place in the kept order (a box
    /// moved out of a removed row keeps its slot).
    InsertedHasPlace,
    /// The previous kept sibling holds a scroll container in flow, whose
    /// thumb's place is not read.
    ScrollInPreviousSibling,
    /// The owner's kept children are all out of flow.
    OnlyOutOfFlowKept,
}
