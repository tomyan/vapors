//! What the update that made a kept order walked, spliced and
//! relabelled (`kept_order.rs`), for `UpdateCounts`.

use crate::splice_decline::SpliceDecline;

/// Whole-tree walks of paint order and chains, and boxes whose clips
/// were redone.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct OrderWalks {
    pub(crate) order: usize,
    pub(crate) chains: usize,
    pub(crate) clips: usize,
    /// Shape changes spliced into the kept order (`order_splice.rs`).
    pub(crate) splices: usize,
    /// Why the splice declined a shape change, walked instead.
    pub(crate) declined: Option<SpliceDecline>,
    /// Labels a splice relabelled to make room (`paint_labels.rs`), and
    /// times it relabelled every label.
    pub(crate) relabelled: usize,
    pub(crate) relabel_all: usize,
}

impl OrderWalks {
    /// Whether paint order was walked or spliced: the one answer to
    /// whether the drawing steps and the scroll containers can differ
    /// from the kept ones.
    pub(crate) fn reshaped(&self) -> bool {
        self.order > 0 || self.splices > 0
    }
}
