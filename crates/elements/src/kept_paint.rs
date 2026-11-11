//! The last update's paint, used again (IU3 3c-1, `docs/plans/iu3.md`).
//! An update that built no box, and in which every box has the rect
//! and the thumbs are shown or hidden as when the kept paint was made,
//! paints nothing and shows the kept paint. Everything else paint reads
//! comes from a box's style or content, and a change to either builds
//! that box again (`layout/marks.rs`); a scale change builds every box.

use crate::layout::BoxRect;

/// What the kept paint was made from, besides the boxes.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct PaintedFrom {
    /// Each slot's border box, scroll offsets applied (`placed`).
    pub(crate) rects: Vec<BoxRect>,
    pub(crate) thumbs_shown: bool,
}

impl PaintedFrom {
    /// Whether paint made from `self` shows what a paint made from `now`
    /// would, after an update that built `built` boxes.
    pub(crate) fn shows(&self, now: &Self, built: usize) -> bool {
        built == 0 && self == now
    }
}
