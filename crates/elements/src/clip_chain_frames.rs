//! The boxes whose clip chain is past `MAX_CLIP_CHAIN` (`clip_chain.rs`),
//! found the same way in every build. A box's chain is its own clip and
//! its frame's links, and a box whose own clip or frame changed is
//! painted again (`BoxFrom` holds both). A frame's links change when a
//! box around it starts or stops scrolling, or a framed lane's container
//! starts or stops clipping, which can leave the boxes in it kept. So
//! each update measures every frame's links, O(frames), and keeps them:
//! when they equal the last update's, the boxes painted again are
//! measured; when any differs, every painted box is.

use crate::clip_chain::{frame_links, links, under};
use crate::frames::Frames;
use crate::kept_order::KeptOrder;
use crate::painted::Painted;

/// Each frame's links: (0, a scroll container's slot) or (1, a framed
/// lane's container's slot) and the links of its content's frame,
/// sorted.
pub(crate) type FrameChains = Vec<(u8, usize, usize)>;

/// The (slot, links) of the boxes past `MAX_CLIP_CHAIN`, of those
/// [`to_measure`] names.
pub(crate) fn long_chains(
    order: &KeptOrder,
    painted: &mut Painted,
    again: &[usize],
) -> Vec<(usize, usize)> {
    let slots = to_measure(order, painted, again);
    let measured = |slot: usize| {
        let links = links(order, &painted.frames, slot);
        (links > crate::MAX_CLIP_CHAIN).then_some((slot, links))
    };
    slots.into_iter().filter_map(measured).collect()
}

/// The boxes painted `again`, or every painted box when a frame's links
/// changed since `painted` was last measured; `painted` keeps this
/// update's frame links.
fn to_measure(order: &KeptOrder, painted: &mut Painted, again: &[usize]) -> Vec<usize> {
    let chains = frame_chains(order, &painted.frames);
    let every = chains != painted.frame_chains;
    painted.frame_chains = chains;
    if !every {
        return again.to_vec();
    }
    let slots = 0..painted.kept_boxes.len();
    slots
        .filter(|&slot| painted.kept_boxes[slot].is_some())
        .collect()
}

/// Every frame's links, sorted.
pub(crate) fn frame_chains(order: &KeptOrder, frames: &Frames) -> FrameChains {
    let (scrolling, lanes) = frames.containers();
    let scroll = scrolling
        .into_iter()
        .map(|slot| (0, slot, frame_links(order, frames, slot)));
    let lane = lanes.into_iter().map(|container| {
        let (link, outer) = under(order, frames, container);
        (
            1,
            container,
            usize::from(link) + frame_links(order, frames, outer),
        )
    });
    let mut chains: FrameChains = scroll.chain(lane).collect();
    chains.sort_unstable();
    chains
}

#[cfg(test)]
#[path = "clip_chain_frames_tests.rs"]
mod tests;
