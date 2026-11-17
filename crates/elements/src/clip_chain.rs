//! The links of the clip chain a box paints under (flat-scene F3,
//! `docs/plans/flat-scene-f3.md` item 6): its own clip, folded within
//! its frame, and one link per enclosing frame boundary under a clip (a
//! scroll container's content clip, or the clip a lane frame's row is
//! under), each in the frame outside it, so a scroll moves none. F3's
//! clip shader walks these links up to [`MAX_CLIP_CHAIN`]; paint
//! refuses a box with more (`paint_refusals.rs`).

use crate::frames::{Frames, PAGE};
use crate::kept_order::KeptOrder;

pub use vapors_scene::MAX_CLIP_CHAIN;

/// The links of the clip chain box `slot` paints under.
pub(crate) fn links(order: &KeptOrder, frames: &Frames, slot: usize) -> usize {
    let own = usize::from(order.clips.own(slot).is_some());
    let mut links = own;
    let mut frame = order.clips.frame(slot);
    while frame != PAGE {
        let (link, outer) = boundary(order, frames, frame);
        links += usize::from(link);
        frame = outer;
    }
    links
}

/// Whether the boundary of `frame` (its container's slot) is under a
/// clip, and the frame outside it. A scroll container clips its content
/// and sits in the frame outside; a lane frame's row is its own frame
/// (`kept_clips.rs`), under its parent's content clip, in the frame its
/// parent's content is in.
fn boundary(order: &KeptOrder, frames: &Frames, frame: usize) -> (bool, usize) {
    let clips = &order.clips;
    if clips.frame(frame) != frame {
        return (clips.inner(frame).is_some(), clips.frame(frame));
    }
    let Some(parent) = order.tree.parent(frame) else {
        return (false, PAGE);
    };
    let outer = if frames.get(parent).is_some() {
        parent
    } else {
        clips.frame(parent)
    };
    (clips.inner(parent).is_some(), outer)
}

impl crate::painted::Painted {
    /// The most links of any painted box's clip chain, for the
    /// measurement [`MAX_CLIP_CHAIN`] cites.
    #[doc(hidden)]
    #[must_use]
    pub fn longest_clip_chain(&self) -> usize {
        let slots = 0..self.order.clips.len();
        slots
            .filter(|&slot| self.kept_boxes.get(slot).is_some_and(Option::is_some))
            .map(|slot| links(&self.order, &self.frames, slot))
            .max()
            .unwrap_or(0)
    }
}
