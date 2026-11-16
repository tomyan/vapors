//! Scroll frames (IU4, `docs/plans/iu4.md` section 3): each part is
//! painted in the coordinates of its frame, the nearest scroll
//! container above it on its clip chain, or the page's for a part under
//! none. Each frame's shift (device pixels, whole) and clip (its
//! scroll container's content clip, on the page) are kept here, per
//! update, and every reader of a painted position adds them on read:
//! the scene's items, the hits, the border boxes and origins.

use rustc_hash::FxHashMap;
use vapors_scene::{Rect, TransformClass};

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
    /// Its composed transform to the target, which decides whether its
    /// items are snapped (`docs/plans/flat-scene.md` section 4.3): each
    /// frame takes the class of the frame it is in, and the page's is a
    /// whole translation, so every frame's is until transforms (F4a).
    pub(crate) class: TransformClass,
}

/// Each frame of a paint, by its scroll container's slot; the scroll
/// containers in each frame, listed and indexed by their clips
/// (`frames_nested.rs`); and the clip each container was indexed with.
#[derive(Debug, PartialEq)]
pub(crate) struct Frames {
    scale: f32,
    by_frame: FxHashMap<usize, FrameAt>,
    /// The scroll frames under a lane frame, by their own part, composed
    /// on read (`frames_relative.rs`); none of them is in `by_frame`.
    relative: FxHashMap<usize, Relative>,
    nested: FxHashMap<usize, Vec<usize>>,
    /// The frame each scroll container was in when these were made.
    outer: FxHashMap<usize, usize>,
    index: FxHashMap<usize, NestedIndex>,
    entered: FxHashMap<usize, Option<Rect>>,
    /// The lane frames, resolved on read (`frames_lane.rs`).
    lanes: LaneFrames,
    /// By framed lane container, how far its children's ink reaches
    /// past their border boxes above and below, CSS pixels: the most
    /// seen since paint order was walked, which culling widens by
    /// (`frames_lane.rs`).
    lane_ink: LaneInk,
}

impl Default for Frames {
    fn default() -> Self {
        Self::page(1.0)
    }
}

impl Frames {
    /// Device pixels per CSS pixel.
    pub(crate) fn scale(&self) -> f32 {
        self.scale
    }

    /// The page's frame alone, at `scale`.
    pub(crate) fn page(scale: f32) -> Self {
        let page = FrameAt {
            shift: [0.0; 2],
            clip: None,
            device_clip: None,
            class: TransformClass::WholeTranslate,
        };
        Self {
            scale,
            by_frame: std::iter::once((PAGE, page)).collect(),
            relative: FxHashMap::default(),
            nested: FxHashMap::default(),
            outer: FxHashMap::default(),
            index: FxHashMap::default(),
            entered: FxHashMap::default(),
            lanes: LaneFrames::default(),
            lane_ink: LaneInk::default(),
        }
    }

    /// Frame `frame`, a scroll frame or a lane frame, if it is one.
    pub(crate) fn get(&self, frame: usize) -> Option<FrameAt> {
        self.primed_at(frame)
            .or_else(|| self.by_frame.get(&frame).copied())
            .or_else(|| self.relative_at(frame))
            .or_else(|| self.lane_at(frame))
    }

    /// Frame `frame`, a scroll frame or a lane frame, or a panic naming
    /// it.
    pub(crate) fn at(&self, frame: usize) -> FrameAt {
        self.get(frame).unwrap_or_else(|| {
            panic!(
                "a part names frame {frame}, which this paint does not hold: the scroll \
                 frames are made from the scroll containers in paint order (frames.rs, \
                 paint_partial.rs), the lane frames from the framed lanes (frames_lane.rs, \
                 paint_frames.rs), and a part's frame from its clip chain (kept_clips.rs); \
                 reproduce with `cargo xtask test`"
            )
        })
    }

    /// Each scroll frame, by its scroll container's slot, those under a
    /// lane frame composed afresh (for the debug checks).
    pub(crate) fn scrollers(&self) -> impl Iterator<Item = (usize, FrameAt)> + '_ {
        let frames = self.by_frame.iter().filter(|&(&frame, _)| frame != PAGE);
        let relative = self.relative.keys().map(|&frame| {
            let at = self.relative_afresh(frame).expect("a relative frame");
            (frame, at)
        });
        frames.map(|(&frame, &at)| (frame, at)).chain(relative)
    }

    /// Frame `frame` at `shift` under the page clip `clip`, its device
    /// clip snapped from it, in a frame of class `class`.
    fn frame_at(
        &self,
        (frame, class): (usize, TransformClass),
        shift: [f32; 2],
        clip: Option<BoxRect>,
    ) -> FrameAt {
        #[cfg(debug_assertions)]
        assert_whole_shift(frame, class, shift);
        let _ = frame;
        FrameAt {
            shift,
            clip,
            device_clip: clip.map(|clip| crate::paint::snapped(clip, self.scale)),
            class,
        }
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
}

/// Panics, naming frame `frame`, when it is of a class that snaps and
/// its shift is not whole device pixels: its items, snapped in it, would
/// land between pixels on the page.
#[cfg(debug_assertions)]
pub(super) fn assert_whole_shift(frame: usize, class: TransformClass, shift: [f32; 2]) {
    assert!(
        !class.snaps() || shift.iter().all(|v| v.fract() == 0.0),
        "frame {frame} is shifted by {shift:?} device pixels, not whole, under a whole \
         translation: its snapped items land between pixels on the page (scroll.rs \
         `frame_shift`, frames_lane_prime.rs `lane_child_at`); reproduce with \
         `cargo xtask test`"
    );
}

/// `inner` cut to `outer`, either `None` for none.
fn meet(inner: Option<BoxRect>, outer: Option<BoxRect>) -> Option<BoxRect> {
    match (inner, outer) {
        (Some(inner), outer @ Some(_)) => Some(crate::clip::within(outer, inner)),
        (inner, outer) => inner.or(outer),
    }
}

/// `boxes`, every part on the page with no shift or clip, for tests.
#[cfg(test)]
pub(crate) fn on_page(
    boxes: &[Option<crate::kept_box::KeptBox>],
) -> (&[Option<crate::kept_box::KeptBox>], &'static Frames) {
    thread_local! {
        // Leaked once per test thread: `Frames` holds the lanes, which
        // are not shared between threads.
        static ON_PAGE: &'static Frames = Box::leak(Box::new(Frames::page(1.0)));
    }
    (boxes, ON_PAGE.with(|frames| *frames))
}

#[path = "frames_nested.rs"]
mod nested;
use nested::NestedIndex;

#[path = "frames_update.rs"]
mod update;

#[path = "frames_compare.rs"]
mod compare;

#[path = "frames_relative.rs"]
mod relative;
use relative::Relative;

#[path = "frames_lane.rs"]
mod lane;
#[path = "frames_read.rs"]
mod read;
pub(crate) use lane::LaneBase;
pub(crate) use lane::LaneSeen;
use lane::{LaneFrames, LaneInk};
#[cfg(debug_assertions)]
pub(crate) use read::shifted;
pub(crate) use update::FrameInputs;

#[cfg(test)]
#[path = "frames_class_tests.rs"]
mod class_tests;
#[cfg(test)]
#[path = "frames_tests.rs"]
mod tests;
