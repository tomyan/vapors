//! Scroll frames under a lane frame (`docs/plans/splice-layout.md`,
//! section 11.2, 4e): a scroll frame whose outer frame is a lane frame,
//! or such a scroll frame, keeps its own part only, its scroll shift and
//! the clip it gives its content in its outer frame's coordinates. Its
//! shift and clip on the page are composed when read, from its outer
//! frame's, and memoised with the lane frames until the next update
//! (`Frames::set_lanes`): an insert above the row changes no stored
//! frame, and a scroll inside it changes that frame's own part alone.

use super::{FrameAt, Frames, meet};
use crate::layout::BoxRect;

/// A scroll frame's own part, in its outer frame's coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Relative {
    /// The frame its scroll container is in: a lane frame, or a scroll
    /// frame kept relative.
    pub(crate) outer: usize,
    /// Its scroll shift, device pixels, whole.
    pub(crate) own: [f32; 2],
    /// The clip its container gives its content, CSS pixels in `outer`.
    pub(crate) inner: Option<BoxRect>,
}

impl Frames {
    /// Whether a scroll frame in frame `outer` keeps its own part only:
    /// `outer` is a lane frame, or a scroll frame kept so.
    pub(super) fn under_lane(&self, outer: usize, lane_frame: &dyn Fn(usize) -> bool) -> bool {
        lane_frame(outer) || self.relative.contains_key(&outer)
    }

    /// Notes scroll frame `slot`'s own part; whether it differs from what
    /// was noted.
    pub(super) fn insert_relative(&mut self, slot: usize, part: Relative) -> bool {
        self.by_frame.remove(&slot);
        self.relative.insert(slot, part) != Some(part)
    }

    /// Scroll frame `frame` on the page, when it is kept relative:
    /// composed from its outer frame, memoised until the next update.
    pub(super) fn relative_at(&self, frame: usize) -> Option<FrameAt> {
        let part = *self.relative.get(&frame)?;
        Some(self.memoised(frame, || self.composed(frame, part, self.at(part.outer))))
    }

    /// Scroll frame `frame` composed from its outer frame worked out
    /// afresh, reading and filling no memo, for the debug checks.
    pub(super) fn relative_afresh(&self, frame: usize) -> Option<FrameAt> {
        let part = *self.relative.get(&frame)?;
        Some(self.composed(frame, part, self.resolved_afresh(part.outer)))
    }

    /// Frame `frame`, of own part `part` in a frame at `outer`, on the
    /// page.
    fn composed(&self, frame: usize, part: Relative, outer: FrameAt) -> FrameAt {
        let shift = [outer.shift[0] + part.own[0], outer.shift[1] + part.own[1]];
        let inner = part.inner.map(|clip| self.css_rect(outer, clip));
        self.frame_at((frame, outer.class), shift, meet(inner, outer.clip))
    }
}

#[cfg(test)]
#[path = "frames_relative_tests.rs"]
mod tests;
