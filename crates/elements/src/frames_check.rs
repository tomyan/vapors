//! The checks that the scroll frames have one answer (`frames.rs`):
//! paint order's thumb steps, which frames are made for, and the
//! clips, which put each box in a frame, name the same scroll
//! containers; and paint's shift for each frame, summed up the clip
//! chain, is placement's, summed up the containing-block chain.

use crate::frames::{Frames, PAGE};
use crate::kept_clips::Clips;
use crate::kept_paint::PaintedFrom;
use crate::layout::Lanes;
use crate::placement_check_page::{chain_shift, lane_offset};

/// Panics, naming the slot, when the two answers to "which boxes are
/// scroll frames" disagree: the scroll containers frames are made for
/// (`scrollers`, paint order's thumb steps) must each scroll, and in a
/// debug build every frame `clips` names must be one of them.
pub(crate) fn assert_scrollers_agree(
    scrollers: &[usize],
    clips: &Clips,
    (scrolls, lane_frame): (&dyn Fn(usize) -> bool, &dyn Fn(usize) -> bool),
) {
    if let Some(slot) = scrollers.iter().find(|&&slot| !scrolls(slot)) {
        panic!(
            "slot {slot} has a thumb step in paint order but does not scroll: paint order \
             (paint_order.rs) and the stackings disagree on the scroll containers; \
             reproduce with `cargo xtask test`"
        );
    }
    if cfg!(debug_assertions) {
        let named = (0..clips.len()).map(|slot| (slot, clips.frame(slot)));
        let stray = named
            .filter(|&(_, frame)| frame != PAGE && !lane_frame(frame))
            .find(|(_, frame)| !scrollers.contains(frame));
        if let Some((slot, frame)) = stray {
            panic!(
                "slot {slot} is in frame {frame}, which has no thumb step in paint order and \
                 is no lane frame: the clips (kept_clips.rs), paint order and placement's lane \
                 frames (placement_frames.rs) disagree; reproduce with `cargo xtask test`"
            );
        }
    }
}

/// In a debug build, panics, naming the frame, when paint's shift for
/// a scroll frame in `frames` is not the one the chain of `from`'s
/// placement gives (`placement_check_page.rs` `chain_shift`): the two
/// chains must put each frame's content in one place on the page.
// Exact: both are sums of whole device pixels.
#[allow(clippy::float_cmp)]
pub(crate) fn assert_frames_follow_placement(
    frames: &Frames,
    from: &PaintedFrom,
    (scrolls, lanes): (&dyn Fn(usize) -> bool, &Lanes),
) {
    if !cfg!(debug_assertions) {
        return;
    }
    let placed = |frame| chain_shift(&from.placement, scrolls, from.scale, frame);
    let mut shifts = frames
        .scrollers()
        .map(|(frame, at)| (frame, at.shift, placed(frame)));
    if let Some((frame, painted, placed)) = shifts.find(|(_, painted, placed)| painted != placed) {
        panic!(
            "scroll frame {frame} is shifted {painted:?} by paint (paint_partial.rs `frames`, \
             up the clip chain) but {placed:?} by placement's chain (placement_check_page.rs \
             `chain_shift`): the chains name different scroll containers above it, or one does \
             not shift by `scroll::frame_shift`; reproduce with `cargo xtask test`"
        );
    }
    assert_lane_frames_follow_placement(frames, from, (scrolls, lanes));
}

/// Panics, naming the lane frame, when paint's shift for a framed lane
/// child (`frames_lane.rs`) is not placement's: the scroll frames' shift
/// up its chain and its lane offset (`placement_check_page.rs`
/// `lane_offset`). O(slots), in a debug build only.
// Exact: both are sums of whole device pixels.
#[allow(clippy::float_cmp)]
fn assert_lane_frames_follow_placement(
    frames: &Frames,
    from: &PaintedFrom,
    (scrolls, lanes): (&dyn Fn(usize) -> bool, &Lanes),
) {
    let placement = &from.placement;
    let lane_children = (0..placement.framed.len()).filter(|&slot| placement.framed[slot]);
    let chained = |slot: usize| {
        let [x, y] = chain_shift(placement, scrolls, from.scale, placement.up[slot]);
        [x, y + lane_offset(placement, lanes, slot) * from.scale]
    };
    let mut shifts = lane_children.map(|slot| (slot, frames.at(slot).shift, chained(slot)));
    if let Some((frame, painted, placed)) = shifts.find(|(_, painted, placed)| painted != placed) {
        panic!(
            "lane frame {frame} is shifted {painted:?} by paint (frames_lane.rs, from its \
             lane's base) but {placed:?} by placement's chain (placement_check_page.rs \
             `lane_offset`): the base's origin or the lane's start disagree with placement; \
             reproduce with `cargo xtask test`"
        );
    }
}

/// In a debug build, panics, naming the first frame that differs, when
/// the frames kept and updated (`paint_frames.rs`) are not those made
/// afresh by `fresh`: an update changed a frame it did not work out
/// again.
pub(crate) fn assert_kept_frames_fresh(kept: &Frames, fresh: impl FnOnce() -> Frames) {
    if !cfg!(debug_assertions) {
        return;
    }
    if let Some(differs) = kept.first_differing(&fresh()) {
        panic!(
            "the kept scroll frames differ from frames made afresh: {differs}; the update did \
             not work the frame out again (paint_frames.rs takes the scroll containers whose \
             offset moved or whose clips were redone); reproduce with `cargo xtask test`"
        );
    }
}
