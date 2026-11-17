//! The links of the clip chain a box paints under (flat-scene F3,
//! `docs/plans/flat-scene-f3.md` item 6): its own clip, folded within
//! its frame, and one link per enclosing frame boundary under a clip (a
//! scroll container's content clip, or the clip a lane frame's row is
//! under), each in the frame outside it, so a scroll moves none. F3's
//! clip shader walks these links up to [`MAX_CLIP_CHAIN`]; paint
//! refuses a box with more (`paint_refusals.rs`).

use crate::frames::{Frames, PAGE};
use crate::kept_order::KeptOrder;

/// The most links a box's clip chain may have. Measured 2026-11-17 at
/// each page's first update at one viewport (the corpus at 800x600, the
/// demos at 640x400): the longest chain in the corpus is
/// [`MEASURED_CORPUS_CLIP_CHAIN`] links (`scrolling`) and in the demos
/// [`MEASURED_DEMOS_CLIP_CHAIN`] (`scroll`), each pinned by its crate's
/// `clip_chains.rs`. touche's own pages are not in either set; it meets
/// this limit as a refusal at its next vapors pin bump. 8 leaves room for
/// scroll containers nested a few deep and for F3.2's rounded clips,
/// each a link of its own. The clip shader's loop bound is this number
/// (F3.1's GPU side).
pub const MAX_CLIP_CHAIN: usize = 8;

/// The longest clip chain in the corpus, measured (`MAX_CLIP_CHAIN`).
#[doc(hidden)]
pub const MEASURED_CORPUS_CLIP_CHAIN: usize = 2;

/// The longest clip chain in the demos, measured (`MAX_CLIP_CHAIN`).
#[doc(hidden)]
pub const MEASURED_DEMOS_CLIP_CHAIN: usize = 2;

/// The links of the clip chain box `slot` paints under: its own clip
/// and its frame's links.
pub(crate) fn links(order: &KeptOrder, frames: &Frames, slot: usize) -> usize {
    let own = usize::from(order.clips.own(slot).is_some());
    own + frame_links(order, frames, order.clips.frame(slot))
}

/// The links of the frame boundaries from `frame` out to the page. Each
/// step goes to an enclosing frame, so the walk takes at most one step
/// a slot; a walk past that panics, naming the frames that loop.
pub(crate) fn frame_links(order: &KeptOrder, frames: &Frames, mut frame: usize) -> usize {
    let (mut links, mut steps) = (0, 0);
    while frame != PAGE {
        steps += 1;
        assert!(steps <= order.clips.len(), "{}", looped(frame));
        let (link, outer) = boundary(order, frames, frame);
        links += usize::from(link);
        frame = outer;
    }
    links
}

fn looped(frame: usize) -> String {
    format!(
        "the clip chain through frame {frame} never reaches the page: each box's frame \
         (kept_clips.rs) must be a box enclosing it, and frame {frame}'s chain comes back to \
         it; fix the frames kept_clips.rs keeps; reproduce with `cargo xtask test`"
    )
}

/// Whether the boundary of `frame` (its container's slot) is under a
/// clip, and the frame outside it. A scroll container clips its content
/// and sits in the frame outside; a lane frame's row is its own frame
/// (`kept_clips.rs`), under its parent's content clip.
fn boundary(order: &KeptOrder, frames: &Frames, frame: usize) -> (bool, usize) {
    let clips = &order.clips;
    if clips.frame(frame) != frame {
        return (clips.inner(frame).is_some(), clips.frame(frame));
    }
    let parent = order.tree.parent(frame).unwrap_or_else(|| {
        panic!(
            "lane frame {frame} has no parent box: a lane row is a child of its lane's \
             container (kept_clips.rs, frames_lane.rs); fix the frame kept for slot {frame}; \
             reproduce with `cargo xtask test`"
        )
    });
    under(order, frames, parent)
}

/// Whether a framed lane's rows under `container` are under its content
/// clip, and the frame its content is in.
pub(crate) fn under(order: &KeptOrder, frames: &Frames, container: usize) -> (bool, usize) {
    let clips = &order.clips;
    let outer = if frames.get(container).is_some() {
        container
    } else {
        clips.frame(container)
    };
    (clips.inner(container).is_some(), outer)
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

#[cfg(test)]
#[path = "clip_chain_tests.rs"]
mod tests;
