//! The check that the scroll frames have one answer (`frames.rs`):
//! paint order's thumb steps, which frames are made for, and the
//! clips, which put each box in a frame, name the same scroll
//! containers.

use crate::frames::PAGE;
use crate::kept_clips::Clips;

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
