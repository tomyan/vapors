//! Comparing kept scroll frames with frames made afresh (`frames.rs`,
//! `frames_check.rs`): which frame, nested list or indexed clip differs
//! first; and whether two sets of frames put anything in another place.

use super::update::bits;
use super::{Frames, NestedIndex};

impl Frames {
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
        self.first_differing_index(other)
    }

    /// The first frame whose nested containers or their index differ, or
    /// the first container entered with another clip, with both.
    fn first_differing_index(&self, other: &Self) -> Option<String> {
        let frames = sorted(self.nested.keys().chain(other.nested.keys()));
        if let Some(at) = frames
            .into_iter()
            .find(|&at| lists(self, at) != lists(other, at))
        {
            let (kept, fresh) = (lists(self, at), lists(other, at));
            return Some(format!(
                "frame {at}'s nested containers: kept {kept:?}, fresh {fresh:?}"
            ));
        }
        let entered = |frames: &Self, slot| frames.entered.get(&slot).map(|&clip| bits(clip));
        let slots = sorted(self.entered.keys().chain(other.entered.keys()));
        let slot = slots
            .into_iter()
            .find(|&slot| entered(self, slot) != entered(other, slot))?;
        let (kept, fresh) = (self.entered.get(&slot), other.entered.get(&slot));
        Some(format!(
            "container {slot} entered with {kept:?}, fresh {fresh:?}"
        ))
    }

    /// Whether every frame's shift and clip, and the scale, are
    /// `other`'s: whether a change moved anything on the page.
    pub(crate) fn same_frames(&self, other: &Self) -> bool {
        self.scale.to_bits() == other.scale.to_bits() && self.by_frame == other.by_frame
    }
}

/// The scroll containers in frame `at` of `frames`, and their index.
fn lists(frames: &Frames, at: usize) -> (Option<&Vec<usize>>, Option<&NestedIndex>) {
    (frames.nested.get(&at), frames.index.get(&at))
}

/// `slots`, sorted, each once.
fn sorted<'a>(slots: impl Iterator<Item = &'a usize>) -> Vec<usize> {
    let mut slots: Vec<usize> = slots.copied().collect();
    slots.sort_unstable();
    slots.dedup();
    slots
}
