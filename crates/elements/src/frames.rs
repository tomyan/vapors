//! Scroll frames (IU4, `docs/plans/iu4.md` section 3): each part is
//! painted in the coordinates of its frame, the nearest scroll
//! container above it on its clip chain, or the page's for a part under
//! none. Each frame's shift (device pixels, whole) and clip (its
//! scroll container's content clip, on the page) are kept here, per
//! update, and every reader of a painted position adds them on read:
//! the scene's items, the hits, the border boxes and origins.

use rustc_hash::FxHashMap;
use vapors_scene::{Item, Rect};

use crate::layout::BoxRect;

/// The page's frame: the parts under no scroll container.
pub(crate) const PAGE: usize = usize::MAX;

/// Where one frame's parts are on the page.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FrameAt {
    /// Added to a part's positions, device pixels, whole.
    pub(crate) shift: [f32; 2],
    /// The clip every part of the frame is under on the page, CSS
    /// pixels, as hits are cut (`paint.rs` `clip_to`); `None` for none.
    pub(crate) clip: Option<BoxRect>,
    /// `clip` in device pixels, as items are cut.
    pub(crate) device_clip: Option<Rect>,
}

/// Each frame of a paint, by its scroll container's slot; the scroll
/// containers in each frame, listed and indexed by their clips
/// (`frames_nested.rs`); and the clip each container was indexed with.
#[derive(Debug, PartialEq)]
pub(crate) struct Frames {
    scale: f32,
    by_frame: FxHashMap<usize, FrameAt>,
    nested: FxHashMap<usize, Vec<usize>>,
    index: FxHashMap<usize, NestedIndex>,
    entered: FxHashMap<usize, Option<Rect>>,
}

impl Default for Frames {
    fn default() -> Self {
        Self::page(1.0)
    }
}

impl Frames {
    /// The page's frame alone, at `scale`.
    pub(crate) fn page(scale: f32) -> Self {
        let page = FrameAt {
            shift: [0.0; 2],
            clip: None,
            device_clip: None,
        };
        Self {
            scale,
            by_frame: std::iter::once((PAGE, page)).collect(),
            nested: FxHashMap::default(),
            index: FxHashMap::default(),
            entered: FxHashMap::default(),
        }
    }

    /// Frame `frame`, or a panic naming it.
    pub(crate) fn at(&self, frame: usize) -> &FrameAt {
        self.by_frame.get(&frame).unwrap_or_else(|| {
            panic!(
                "a part names scroll frame {frame}, which this paint does not hold: the \
                 frames are made from the scroll containers in paint order (frames.rs, \
                 paint_partial.rs) and a part's frame from its clip chain (kept_clips.rs); \
                 reproduce with `cargo xtask test`"
            )
        })
    }

    /// Each frame but the page's, by its scroll container's slot.
    pub(crate) fn scrollers(&self) -> impl Iterator<Item = (usize, &FrameAt)> {
        let frames = self.by_frame.iter().filter(|&(&frame, _)| frame != PAGE);
        frames.map(|(&frame, at)| (frame, at))
    }

    /// Every frame but the page's shifted by `shift` more, device
    /// pixels: what a scroll does to its container's frame, for tests
    /// of the readers before placement scrolls frames.
    #[cfg(test)]
    pub(crate) fn shift_every_frame(&mut self, [dx, dy]: [f32; 2]) {
        for (&frame, at) in &mut self.by_frame {
            if frame != PAGE {
                at.shift = [at.shift[0] + dx, at.shift[1] + dy];
            }
        }
    }

    /// `rect`, CSS pixels in frame `frame`, on the page.
    pub(crate) fn rect(&self, frame: usize, rect: BoxRect) -> BoxRect {
        self.css_rect(*self.at(frame), rect)
    }

    /// `point`, CSS pixels in frame `frame`, on the page.
    pub(crate) fn point(&self, frame: usize, [x, y]: [f32; 2]) -> [f32; 2] {
        let [dx, dy] = self.css_shift(*self.at(frame));
        [x + dx, y + dy]
    }

    /// A hit `rect` in frame `frame` on the page, cut to the frame's
    /// clip.
    pub(crate) fn hit(&self, frame: usize, rect: BoxRect) -> BoxRect {
        let at = *self.at(frame);
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
        clip: Option<Rect>,
    ) -> (Item, Option<Rect>) {
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
            (Some(clip), Some(frame)) => Some(clip.intersection(frame)),
            (clip, frame) => clip.or(frame),
        };
        (item, clip)
    }

    /// `rect` in a frame at `at`, on the page.
    fn css_rect(&self, at: FrameAt, rect: BoxRect) -> BoxRect {
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

/// `inner` cut to `outer`, either `None` for none.
fn meet(inner: Option<BoxRect>, outer: Option<BoxRect>) -> Option<BoxRect> {
    match (inner, outer) {
        (Some(inner), outer @ Some(_)) => Some(crate::clip::within(outer, inner)),
        (inner, outer) => inner.or(outer),
    }
}

/// `item` moved by `delta`, device pixels.
fn shifted(item: Item, delta: [f32; 2]) -> Item {
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

/// `boxes`, every part on the page with no shift or clip, for tests.
#[cfg(test)]
pub(crate) fn on_page(
    boxes: &[Option<crate::kept_box::KeptBox>],
) -> (&[Option<crate::kept_box::KeptBox>], &'static Frames) {
    static ON_PAGE: std::sync::LazyLock<Frames> = std::sync::LazyLock::new(|| Frames::page(1.0));
    (boxes, &ON_PAGE)
}

#[path = "frames_nested.rs"]
mod nested;
use nested::NestedIndex;

#[path = "frames_update.rs"]
mod update;

#[path = "frames_compare.rs"]
mod compare;
pub(crate) use update::FrameInputs;

#[cfg(test)]
#[path = "frames_tests.rs"]
mod tests;
