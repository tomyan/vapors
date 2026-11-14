//! Whether a lane's children can be frames of their own
//! (`docs/plans/splice-layout.md`, section 11.1): a lane frame's shift
//! past its scroll frame is the container's content origin, the lane's
//! top and the child's start, and a frame's shift is whole device
//! pixels, so every one of those must be whole at the scale; and no
//! child may hold a scroll container, whose frame is worked out from
//! the frame it is in before a lane frame is resolved.

use rustc_hash::FxHashSet;
use taffy::Layout;

use crate::lane_decline::LaneDecline;

/// What a lane knows of its children for framing them, kept as leaves
/// are put and children taken out.
#[derive(Debug, Default)]
pub(crate) struct FrameFacts {
    /// Children whose height or vertical margins are not whole device
    /// pixels, so some later start may not be.
    fractional: FxHashSet<usize>,
    /// Children with a scroll container at or below them.
    scrolling: FxHashSet<usize>,
    /// Where the starts are measured from in the container's border
    /// box (`LaneKind::top`), as last written.
    pub(super) top: f64,
}

impl FrameFacts {
    /// Notes the child in `slot`, laid out as `layout` at `scale`, with
    /// a scroll container at or below it or not.
    pub(super) fn note(&mut self, slot: usize, layout: &Layout, scale: f32, scrolls: bool) {
        let along = [layout.size.height, layout.margin.top, layout.margin.bottom];
        set(
            &mut self.fractional,
            slot,
            !along.iter().all(|&v| whole(f64::from(v), scale)),
        );
        set(&mut self.scrolling, slot, scrolls);
    }

    /// Forgets the child taken out of `slot`.
    pub(super) fn forget(&mut self, slot: usize) {
        self.fractional.remove(&slot);
        self.scrolling.remove(&slot);
    }

    /// Whether the children can be frames, their gap `gap` and the
    /// container's content origin along the axis `origin`, at `scale`.
    ///
    /// # Errors
    /// A child holds a scroll container (`LaneFrameScroller`), or some
    /// start may not be whole (`LaneFramePixel`).
    pub(crate) fn framable(&self, gap: f64, origin: f64, scale: f32) -> Result<(), LaneDecline> {
        if !self.scrolling.is_empty() {
            return Err(LaneDecline::LaneFrameScroller);
        }
        let whole_starts = self.fractional.is_empty() && whole(gap, scale);
        if !whole_starts || !whole(origin + self.top, scale) {
            return Err(LaneDecline::LaneFramePixel);
        }
        Ok(())
    }
}

/// Puts `slot` in `set` or takes it out.
fn set(set: &mut FxHashSet<usize>, slot: usize, member: bool) {
    if member {
        set.insert(slot);
    } else {
        set.remove(&slot);
    }
}

/// Whether `value` CSS pixels is a whole number of device pixels at
/// `scale`.
pub(crate) fn whole(value: f64, scale: f32) -> bool {
    let device = value * f64::from(scale);
    device.is_finite() && device.fract() == 0.0
}

#[cfg(test)]
#[path = "lane_frame_tests.rs"]
mod tests;
