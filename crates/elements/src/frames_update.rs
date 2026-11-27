//! Making the scroll frames (`frames.rs`), and updating them for the
//! scroll containers an update changed: a frame's shift and clip come
//! from the frame it is in, its scroll offset and the clip it gives its
//! content, so only a changed container and the frames nested in it
//! are worked out again. A page of thousands of small scrollers then
//! works out O(changed) frames per edit, not every one.

use rustc_hash::FxHashSet;

use vapors_scene::{Rect, TransformClass};

use super::NestedIndex;
use super::{Frames, Relative, meet};
use crate::kept_clips::Clips;
use crate::layout::BoxRect;
use crate::paint::snapped;

/// What a frame is worked out from: each box's clips and frame, each
/// scroll container's depth, its own shift (device pixels), and which
/// frames are lane frames, as placement decided them, so making a frame
/// resolves no lane frame (section 11.2, 4e).
pub(crate) struct FrameInputs<'a> {
    pub(crate) clips: &'a Clips,
    pub(crate) depth: &'a dyn Fn(usize) -> usize,
    pub(crate) shift: &'a dyn Fn(usize) -> [f32; 2],
    pub(crate) lane_frame: &'a dyn Fn(usize) -> bool,
}

impl Frames {
    /// The frames of the scroll containers `scrollers`, each shifted by
    /// its own shift from the frame it is in and clipped by the clip it
    /// gives its content in that frame, at `scale`. Shallower scroll
    /// containers are worked out first.
    pub(crate) fn of(scrollers: &[usize], inputs: &FrameInputs<'_>, scale: f32) -> Self {
        let mut frames = Self::page(scale);
        let mut ordered = scrollers.to_vec();
        ordered.sort_by_key(|&slot| (inputs.depth)(slot));
        for slot in ordered {
            let outer = inputs.clips.frame(slot);
            frames.nested.entry(outer).or_default().push(slot);
            frames.outer.insert(slot, outer);
            frames.work_out(slot, inputs);
            frames
                .entered
                .insert(slot, frames.clip_entered(slot, inputs));
        }
        for (&outer, slots) in &frames.nested {
            let clips = slots.iter().map(|&slot| (slot, frames.entered[&slot]));
            frames.index.insert(outer, NestedIndex::of(clips));
        }
        frames
    }

    /// `self` with the frames of the scroll containers `changed`, and
    /// of those nested in a frame that came out different, worked out
    /// again; whether any frame came out different, and how many were
    /// worked out. A scroll container `self` was not made for is
    /// refused, naming it: its frame would be in no frame's nested list,
    /// so a scroll of its outer frame would never reach it.
    pub(crate) fn update(&mut self, changed: &[usize], inputs: &FrameInputs<'_>) -> (bool, usize) {
        let mut starts = changed.to_vec();
        starts.sort_by_key(|&slot| (inputs.depth)(slot));
        let mut done = FxHashSet::default();
        let mut moved = false;
        for start in starts {
            self.assert_held(start, inputs);
            self.enter_again(start, inputs);
            moved |= self.descend(start, inputs, &mut done);
        }
        (moved, done.len())
    }

    /// Container `slot` entered again in its frame's index, when its
    /// clip there changed.
    fn enter_again(&mut self, slot: usize, inputs: &FrameInputs<'_>) {
        let clip = self.clip_entered(slot, inputs);
        let was = self.entered.insert(slot, clip).unwrap_or_else(|| {
            panic!(
                "scroll container {slot} is updated but was never entered in its frame's index: \
                 the frames update only the containers they were made for (paint_frames.rs); \
                 reproduce with `cargo xtask test`"
            )
        });
        if bits(was) != bits(clip) {
            let index = self.index.entry(inputs.clips.frame(slot)).or_default();
            index.remove(slot, was);
            index.insert(slot, clip);
        }
    }

    /// Container `slot`'s inner clip, device pixels in the frame it is
    /// in, a device pixel wider on each side: what it is indexed by
    /// there. The frame's own device clip is snapped from the clip moved
    /// onto the page in CSS pixels, which at a fractional scale can land
    /// an edge a pixel from this one plus the shift; the margin keeps
    /// every pixel the frame can show inside the indexed clip. Steps are
    /// still tested exactly, so the margin costs only candidates.
    fn clip_entered(&self, slot: usize, inputs: &FrameInputs<'_>) -> Option<Rect> {
        let snap = |clip| snapped(clip, self.scale);
        let widened = |r: Rect| Rect::new(r.left - 1.0, r.top - 1.0, r.right + 1.0, r.bottom + 1.0);
        inputs
            .clips
            .inner(slot)
            .map(crate::clip::BoxClip::square_rect)
            .map(snap)
            .map(widened)
    }

    /// Works out `start` and, below each frame that came out different,
    /// the frames nested in it, each once (`done`); whether any did.
    fn descend(
        &mut self,
        start: usize,
        inputs: &FrameInputs<'_>,
        done: &mut FxHashSet<usize>,
    ) -> bool {
        let mut moved = false;
        let mut open = vec![start];
        while let Some(slot) = open.pop() {
            self.assert_held(slot, inputs);
            if !done.insert(slot) || !self.work_out(slot, inputs) {
                continue;
            }
            moved = true;
            // the frames in a relative frame keep their own parts too,
            // which its change does not touch
            if !self.relative.contains_key(&slot) {
                open.extend(self.nested.get(&slot).into_iter().flatten().copied());
            }
        }
        moved
    }

    /// Panics, naming `slot`, when these frames were not made for it, or
    /// were made with it in another outer frame: its frame and the
    /// nested lists and indexes it is in would be the old frame's.
    fn assert_held(&self, slot: usize, inputs: &FrameInputs<'_>) {
        let made = self.outer.get(&slot).copied();
        assert!(
            made.is_some(),
            "scroll container slot {slot} is updated but the kept frames were not made for it: \
             the kept frames and the stackings disagree on the scroll containers (paint_frames.rs \
             makes them afresh whenever paint order is walked or spliced); reproduce with \
             `cargo xtask test`"
        );
        let now = inputs.clips.frame(slot);
        assert!(
            made == Some(now),
            "scroll container slot {slot} is updated in frame {now} but the kept frames were made \
             with it in frame {made:?}: a lane's framing flipped with the frames kept \
             (paint_frames.rs makes them afresh when `ChangedSlots::reframed`); reproduce with \
             `cargo xtask test`"
        );
    }

    /// Works out frame `slot` from the frame it is in, or only its own
    /// part under a lane frame; whether it came out different.
    fn work_out(&mut self, slot: usize, inputs: &FrameInputs<'_>) -> bool {
        let outer = inputs.clips.frame(slot);
        assert_ne!(
            outer, slot,
            "scroll frame {slot} would be in itself: it is a lane frame too (frames_check.rs \
             `assert_scrollers_agree`); reproduce with `cargo xtask test`"
        );
        let own = (inputs.shift)(slot);
        if self.under_lane(outer, inputs.lane_frame) {
            let inner = inputs
                .clips
                .inner(slot)
                .map(crate::clip::BoxClip::square_rect);
            return self.insert_relative(slot, Relative { outer, own, inner });
        }
        let outer = self.at(outer);
        let shift = [outer.shift[0] + own[0], outer.shift[1] + own[1]];
        let inner = inputs
            .clips
            .inner(slot)
            .map(|clip| self.css_rect(outer, clip.square_rect()));
        self.insert((slot, outer.class), shift, meet(inner, outer.clip))
    }

    /// Notes frame `slot`, in a frame of class `class`, at `shift` under
    /// the page clip `clip`; whether it differs from what was noted.
    fn insert(
        &mut self,
        (slot, class): (usize, TransformClass),
        shift: [f32; 2],
        clip: Option<BoxRect>,
    ) -> bool {
        let at = self.frame_at((slot, class), shift, clip);
        self.relative.remove(&slot);
        self.by_frame.insert(slot, at) != Some(at)
    }
}

/// `clip`'s edges' bits, for an exact comparison: -0.0 is not 0.0,
/// and a NaN edge is its own.
pub(super) fn bits(clip: Option<Rect>) -> Option<[u32; 4]> {
    clip.map(|r| [r.left, r.top, r.right, r.bottom].map(f32::to_bits))
}

#[cfg(test)]
#[path = "frames_update_tests.rs"]
mod tests;
