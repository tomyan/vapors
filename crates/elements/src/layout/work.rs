//! What one layout settles besides the boxes, and what it did.

use super::isolated::{Changed, Settled};
use super::lane_layout::LaneCounts;
use super::written::Written;

/// What one layout did: how many times Taffy laid a box out rather than
/// reading its cache, how many times it asked a box for its layout, how
/// the changed owners were settled, and the slots it laid out or wrote.
pub(crate) struct LayoutWork {
    pub(crate) laid_out: usize,
    pub(crate) calls: usize,
    /// Recorded size answers disowned (`tree_cache.rs`).
    pub(crate) disowned: usize,
    pub(crate) settled: Settled,
    /// Containers laid out by a lane, and declined (`lane_layout.rs`).
    pub(crate) lanes: LaneCounts,
    pub(crate) written: Written,
}

/// What a layout settles besides the boxes: the changed owners, the
/// slots built this update, and whether lanes lay containers out.
pub(crate) struct Settling {
    pub(crate) changed: Vec<Changed>,
    pub(crate) built: Vec<usize>,
    pub(crate) lanes: bool,
}

impl Settling {
    /// `changed` to settle, nothing built, lanes on.
    #[cfg(test)]
    pub(crate) fn of(changed: Vec<Changed>) -> Self {
        Self {
            changed,
            built: Vec::new(),
            lanes: true,
        }
    }
}
