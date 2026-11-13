//! Making the scroll frames (`frames.rs`), and updating them for the
//! scroll containers an update changed: a frame's shift and clip come
//! from the frame it is in, its scroll offset and the clip it gives its
//! content, so only a changed container and the frames nested in it
//! are worked out again. A page of thousands of small scrollers then
//! works out O(changed) frames per edit, not every one.

use rustc_hash::FxHashSet;

use super::{FrameAt, Frames, meet};
use crate::kept_clips::Clips;
use crate::layout::BoxRect;
use crate::paint::snapped;

/// What a frame is worked out from: each box's clips and frame, each
/// scroll container's depth, and its own shift (device pixels).
pub(crate) struct FrameInputs<'a> {
    pub(crate) clips: &'a Clips,
    pub(crate) depth: &'a dyn Fn(usize) -> usize,
    pub(crate) shift: &'a dyn Fn(usize) -> [f32; 2],
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
            frames.work_out(slot, inputs);
        }
        frames
    }

    /// `self` with the frames of the scroll containers `changed`, and
    /// of those nested in a frame that came out different, worked out
    /// again; whether any frame came out different, and how many were
    /// worked out. The scroll containers must be those `self` was made
    /// for.
    pub(crate) fn update(&mut self, changed: &[usize], inputs: &FrameInputs<'_>) -> (bool, usize) {
        let mut starts = changed.to_vec();
        starts.sort_by_key(|&slot| (inputs.depth)(slot));
        let mut done = FxHashSet::default();
        let mut moved = false;
        for start in starts {
            let mut open = vec![start];
            while let Some(slot) = open.pop() {
                if !done.insert(slot) || !self.work_out(slot, inputs) {
                    continue;
                }
                moved = true;
                open.extend(self.nested.get(&slot).into_iter().flatten().copied());
            }
        }
        (moved, done.len())
    }

    /// Works out frame `slot` from the frame it is in; whether it came
    /// out different.
    fn work_out(&mut self, slot: usize, inputs: &FrameInputs<'_>) -> bool {
        let outer = *self.at(inputs.clips.frame(slot));
        let own = (inputs.shift)(slot);
        let shift = [outer.shift[0] + own[0], outer.shift[1] + own[1]];
        let inner = inputs
            .clips
            .inner(slot)
            .map(|clip| self.css_rect(outer, clip));
        self.insert(slot, shift, meet(inner, outer.clip))
    }

    /// The first frame that is not the same in `self` and `other`, with
    /// both, or the scroll containers nested in a frame when those
    /// differ.
    pub(crate) fn first_differing(&self, other: &Self) -> Option<String> {
        let frames = self.by_frame.keys().chain(other.by_frame.keys());
        let mut slots: Vec<usize> = frames.copied().collect();
        slots.sort_unstable();
        let differs = |&slot: &usize| self.by_frame.get(&slot) != other.by_frame.get(&slot);
        if let Some(slot) = slots.into_iter().find(differs) {
            let (kept, fresh) = (self.by_frame.get(&slot), other.by_frame.get(&slot));
            return Some(format!("frame {slot}: kept {kept:?}, fresh {fresh:?}"));
        }
        (self.nested != other.nested).then(|| "the scroll containers in each frame".to_owned())
    }

    /// Notes frame `slot` at `shift` under the page clip `clip`; whether
    /// it differs from what was noted.
    fn insert(&mut self, slot: usize, shift: [f32; 2], clip: Option<BoxRect>) -> bool {
        let device_clip = clip.map(|clip| snapped(clip, self.scale));
        let at = FrameAt {
            shift,
            clip,
            device_clip,
        };
        self.by_frame.insert(slot, at) != Some(at)
    }
}

#[cfg(test)]
#[path = "frames_update_tests.rs"]
mod tests;
