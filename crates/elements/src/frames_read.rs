//! Reading painted positions through their frames (`frames.rs`): a rect,
//! a point, a hit or a scene item in a frame's coordinates put on the
//! page by the frame's shift and cut to its clip.

use vapors_scene::{Clip, Item};

use super::{FrameAt, Frames, PAGE};
use crate::layout::BoxRect;

impl Frames {
    /// The slots of the scroll containers whose frames these are, and of
    /// the framed lanes' containers, for the clip chains
    /// (`clip_chain_frames.rs`).
    pub(crate) fn containers(&self) -> (Vec<usize>, Vec<usize>) {
        let scrolling = self.by_frame.keys().filter(|&&frame| frame != PAGE);
        let scrolling = scrolling.chain(self.relative.keys()).copied().collect();
        (scrolling, self.lane_containers())
    }

    /// `rect`, CSS pixels in frame `frame`, on the page.
    pub(crate) fn rect(&self, frame: usize, rect: BoxRect) -> BoxRect {
        let [dx, dy] = self.shift_of(frame).map(|v| v / self.scale);
        BoxRect {
            x: rect.x + dx,
            y: rect.y + dy,
            ..rect
        }
    }

    /// `point`, CSS pixels in frame `frame`, on the page.
    pub(crate) fn point(&self, frame: usize, [x, y]: [f32; 2]) -> [f32; 2] {
        let [dx, dy] = self.shift_of(frame).map(|v| v / self.scale);
        [x + dx, y + dy]
    }

    /// Frame `frame`'s shift, device pixels: none for the page's frame,
    /// which nothing shifts (`shift_every_frame` passes it, `insert`
    /// takes scroll containers' slots); a primed lane frame's from its
    /// table entry; a scroll frame's from its entry; any other's from
    /// `at` (which names a missing frame).
    fn shift_of(&self, frame: usize) -> [f32; 2] {
        if frame == super::PAGE {
            debug_assert_eq!(
                self.by_frame[&super::PAGE].shift,
                [0.0; 2],
                "the page's frame moved"
            );
            return [0.0; 2];
        }
        let scroll = || self.by_frame.get(&frame).map(|at| at.shift);
        self.primed_shift(frame)
            .or_else(scroll)
            .unwrap_or_else(|| self.at(frame).shift)
    }

    /// A hit `rect` in frame `frame` on the page, cut to the frame's
    /// clip.
    pub(crate) fn hit(&self, frame: usize, rect: BoxRect) -> BoxRect {
        self.hit_at(self.at(frame), rect)
    }

    /// A hit `rect` in frame `frame` on the page, when the frame is
    /// still one: a hit taken out with a lane child names the child's
    /// frame, gone with it.
    pub(crate) fn hit_if_held(&self, frame: usize, rect: BoxRect) -> Option<BoxRect> {
        self.get(frame).map(|at| self.hit_at(at, rect))
    }

    fn hit_at(&self, at: FrameAt, rect: BoxRect) -> BoxRect {
        crate::clip::within(at.clip, self.css_rect(at, rect))
    }

    /// `item` under `clip`, device pixels in frame `frame`, on the page
    /// and under the frame's clip too.
    // Exact: a zero shift is exactly zero.
    #[allow(clippy::float_cmp)]
    pub(crate) fn item(
        &self,
        frame: usize,
        item: &Item,
        clip: Option<Clip>,
    ) -> (Item, Option<Clip>) {
        let at = self.at(frame);
        // An unshifted frame keeps every coordinate's bits: adding 0.0
        // turns -0.0 into 0.0, which the kept batches' keys tell apart
        // (app tests/gpu_kept_batches.rs).
        let unshifted = at.shift.iter().all(|&v| v == 0.0);
        let (item, clip) = if unshifted {
            (*item, clip)
        } else {
            (
                shifted(*item, at.shift),
                clip.map(|clip| clip.offset(at.shift)),
            )
        };
        let clip = match (clip, at.device_clip) {
            (Some(clip), Some(frame)) => Some(clip.cut(frame)),
            (clip, frame) => clip.or(frame.map(Clip::square)),
        };
        (item, clip)
    }

    /// `rect` in a frame at `at`, on the page.
    pub(super) fn css_rect(&self, at: FrameAt, rect: BoxRect) -> BoxRect {
        let [dx, dy] = self.css_shift(at);
        BoxRect {
            x: rect.x + dx,
            y: rect.y + dy,
            ..rect
        }
    }

    /// A frame's shift in CSS pixels.
    fn css_shift(&self, at: FrameAt) -> [f32; 2] {
        at.shift.map(|v| v / self.scale)
    }
}

/// `item` moved by `delta`, device pixels.
pub(crate) fn shifted(item: Item, delta: [f32; 2]) -> Item {
    match item {
        Item::Text { text, origin, ink } => Item::Text {
            text,
            origin: [origin[0] + delta[0], origin[1] + delta[1]],
            ink: ink.offset(delta),
        },
        moved @ (Item::Shadow(_)
        | Item::Quad(_)
        | Item::Image { .. }
        | Item::Icon { .. }
        | Item::Surface { .. }
        | Item::Shape(_)) => crate::kept_shift_items::shifted_item(moved, delta),
    }
}
