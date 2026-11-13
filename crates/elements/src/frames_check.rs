//! The checks that the scroll frames have one answer (`frames.rs`):
//! paint order's thumb steps, which frames are made for, and the
//! clips, which put each box in a frame, name the same scroll
//! containers; and paint's shift for each frame, summed up the clip
//! chain, is placement's, summed up the containing-block chain.

use crate::frames::{Frames, PAGE};
use crate::kept_clips::Clips;
use crate::kept_paint::PaintedFrom;
use crate::placement_check_page::chain_shift;

/// Panics, naming the slot, when the two answers to "which boxes are
/// scroll frames" disagree: the scroll containers frames are made for
/// (`scrollers`, paint order's thumb steps) must each scroll, and in a
/// debug build every frame `clips` names must be one of them.
pub(crate) fn assert_scrollers_agree(
    scrollers: &[usize],
    clips: &Clips,
    scrolls: &dyn Fn(usize) -> bool,
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
            .filter(|&(_, frame)| frame != PAGE)
            .find(|(_, frame)| !scrollers.contains(frame));
        if let Some((slot, frame)) = stray {
            panic!(
                "slot {slot} is in scroll frame {frame}, which has no thumb step in paint \
                 order: the clips (kept_clips.rs) and paint order disagree on the scroll \
                 containers; reproduce with `cargo xtask test`"
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
    scrolls: &dyn Fn(usize) -> bool,
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
}
