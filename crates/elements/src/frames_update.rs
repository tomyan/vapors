//! Making the scroll frames (`frames.rs`), and updating them for the
//! scroll containers an update changed: a frame's shift and clip come
//! from the frame it is in, its scroll offset and the clip it gives its
//! content, so only a changed container and the frames nested in it
//! are worked out again. A page of thousands of small scrollers then
//! works out O(changed) frames per edit, not every one.

use rustc_hash::FxHashSet;

use vapors_scene::Rect;

use super::NestedIndex;
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
            self.assert_held(start);
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
        if was != clip {
            let index = self.index.entry(inputs.clips.frame(slot)).or_default();
            index.remove(slot, was);
            index.insert(slot, clip);
        }
    }

    /// Container `slot`'s inner clip, device pixels in the frame it is
    /// in: what it is indexed by there.
    fn clip_entered(&self, slot: usize, inputs: &FrameInputs<'_>) -> Option<Rect> {
        inputs
            .clips
            .inner(slot)
            .map(|clip| snapped(clip, self.scale))
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
            self.assert_held(slot);
            if !done.insert(slot) || !self.work_out(slot, inputs) {
                continue;
            }
            moved = true;
            open.extend(self.nested.get(&slot).into_iter().flatten().copied());
        }
        moved
    }

    /// Panics, naming `slot`, when these frames were not made for it.
    fn assert_held(&self, slot: usize) {
        assert!(
            self.by_frame.contains_key(&slot),
            "scroll container slot {slot} is updated but the kept frames were not made for it: \
             the kept frames and the stackings disagree on the scroll containers (paint_frames.rs \
             makes them afresh whenever paint order is walked or spliced); reproduce with \
             `cargo xtask test`"
        );
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
        if self.scale.to_bits() != other.scale.to_bits() {
            return Some(format!(
                "the scale: kept {}, fresh {}",
                self.scale, other.scale
            ));
        }
        let frames = self.by_frame.keys().chain(other.by_frame.keys());
        let mut slots: Vec<usize> = frames.copied().collect();
        slots.sort_unstable();
        let differs = |&slot: &usize| self.by_frame.get(&slot) != other.by_frame.get(&slot);
        if let Some(slot) = slots.into_iter().find(differs) {
            let (kept, fresh) = (self.by_frame.get(&slot), other.by_frame.get(&slot));
            return Some(format!("frame {slot}: kept {kept:?}, fresh {fresh:?}"));
        }
        let lists = (&self.nested, &self.index, &self.entered);
        (lists != (&other.nested, &other.index, &other.entered)).then(|| {
            format!(
                "the scroll containers in each frame or their index: kept {:?} {:?}, fresh {:?} {:?}",
                self.index, self.entered, other.index, other.entered
            )
        })
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
