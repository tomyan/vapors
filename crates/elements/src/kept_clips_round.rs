//! What `Clips` keeps of rounded clips beside the clips themselves
//! (flat-scene F3.2b, `docs/plans/frame-clips.md` slice 2): each box's
//! refusal, each rounding box's padding box and radii, and the frames in
//! a rounded lane frame, which each update checks against its corners
//! after paint (`clip_refusals.rs` `nested_frames`), as rows move with no
//! box painted again.

use super::{Clips, Inner, InnerClip};
use crate::clip::BoxClip;
use crate::clip_refusals::RoundedRefusal;
use crate::clip_tree::ClipTree;
use crate::frames::PAGE;

impl Clips {
    /// The box rounding frame `frame`, when it is a lane row whose
    /// container's clip is rounded; its container is redone before it. A
    /// row's clip does not take its container's, so a change of rounding
    /// alone would not reach the boxes in it here: it comes from a restyle
    /// of the rounding box, which builds its whole subtree again
    /// (`restyle_pass.rs`, `layout/marks.rs`), each box then redone
    /// (`tests/rounded_clip_frames.rs`, the card rounding after an
    /// update). `Clips::first_differing` holds `nested` and `refused` to a
    /// fresh paint's in a debug build.
    pub(super) fn round_frame(
        &self,
        frame: usize,
        tree: &ClipTree,
        inner: Inner<'_>,
    ) -> Option<usize> {
        if frame == PAGE || !inner.lane_frame(frame) {
            return None;
        }
        let container = tree.parent(frame)?;
        self.inner[container].and_then(|clip: BoxClip| clip.rounded_by())
    }

    /// Notes `made`'s refusal, rounding and nesting for box `slot`; its
    /// clip.
    pub(super) fn note(&mut self, slot: usize, made: InnerClip) -> Option<BoxClip> {
        match made.refused {
            Some(why) => self.refused.insert(slot, why),
            None => self.refused.remove(&slot),
        };
        match made.rounding {
            Some(rounding) => self.rounding.insert(slot, rounding),
            None => self.rounding.remove(&slot),
        };
        match made.nested {
            Some(by) => self.nested.insert(slot, by),
            None => self.nested.remove(&slot),
        };
        made.clip
    }

    /// The frames in a rounded lane frame, with the box rounding it, in
    /// slot order.
    pub(crate) fn nested(&self) -> Vec<(usize, usize)> {
        let mut nested: Vec<_> = self.nested.iter().map(|(&slot, &by)| (slot, by)).collect();
        nested.sort_unstable();
        nested
    }

    /// The slots a rounded clip refuses, and why, in slot order.
    pub(crate) fn refused(&self) -> Vec<(usize, RoundedRefusal)> {
        let mut refused: Vec<_> = self
            .refused
            .iter()
            .map(|(&slot, &why)| (slot, why))
            .collect();
        refused.sort_unstable();
        refused
    }
}
