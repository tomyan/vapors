//! Scroll frames (IU4, `docs/plans/iu4.md` section 3): each part is
//! painted in the coordinates of its frame, the nearest scroll
//! container above it on its clip chain, or the page's for a part under
//! none. Each frame's shift (device pixels, whole) and clip (its
//! scroll container's content clip, on the page) are kept here, per
//! update, and every reader of a painted position adds them on read:
//! the scene's items, the hits, the border boxes and origins.

use rustc_hash::FxHashMap;
use vapors_scene::{Item, Rect};

use crate::kept_clips::Clips;
use crate::layout::BoxRect;
use crate::paint::snapped;

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

/// Each frame of a paint, by its scroll container's slot.
#[derive(Debug, PartialEq)]
pub(crate) struct Frames {
    scale: f32,
    by_frame: FxHashMap<usize, FrameAt>,
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
        }
    }

    /// The frames of the scroll containers `scrollers`, each shifted by
    /// `shift(slot)` from the frame it is in and clipped by the clip it
    /// gives its content in that frame (`clips`), at `scale`. Shallower
    /// scroll containers are worked out first.
    pub(crate) fn of(
        scrollers: &[usize],
        (clips, depth): (&Clips, &dyn Fn(usize) -> usize),
        shift: &dyn Fn(usize) -> [f32; 2],
        scale: f32,
    ) -> Self {
        let mut frames = Self::page(scale);
        let mut ordered = scrollers.to_vec();
        ordered.sort_by_key(|&slot| depth(slot));
        for slot in ordered {
            let outer = *frames.at(clips.frame(slot));
            let own = shift(slot);
            let shift = [outer.shift[0] + own[0], outer.shift[1] + own[1]];
            let inner = clips.inner(slot).map(|clip| frames.css_rect(outer, clip));
            frames.insert(slot, shift, meet(inner, outer.clip));
        }
        frames
    }

    /// Notes frame `slot` at `shift` under the page clip `clip`.
    pub(crate) fn insert(&mut self, slot: usize, shift: [f32; 2], clip: Option<BoxRect>) {
        let device_clip = clip.map(|clip| snapped(clip, self.scale));
        let at = FrameAt {
            shift,
            clip,
            device_clip,
        };
        self.by_frame.insert(slot, at);
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
    pub(crate) fn item(
        &self,
        frame: usize,
        item: &Item,
        clip: Option<Rect>,
    ) -> (Item, Option<Rect>) {
        let at = self.at(frame);
        let clip = clip.map(|clip| clip.offset(at.shift));
        let clip = match (clip, at.device_clip) {
            (Some(clip), Some(frame)) => Some(clip.intersection(frame)),
            (clip, frame) => clip.or(frame),
        };
        (shifted(*item, at.shift), clip)
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
        other => crate::kept_shift_items::shifted_item(other, delta),
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

#[cfg(test)]
#[path = "frames_tests.rs"]
mod tests;
