//! The scroll frames an update paints in (`frames.rs`): made afresh
//! when paint order was walked or spliced, or nothing was kept, which
//! can change the scroll containers; else kept, and worked out again
//! for the scroll containers whose offset moved or whose clips were
//! redone, and the frames nested in them. A debug build compares the
//! kept frames with frames made afresh after every update.

use crate::changed_slots::ChangedSlots;
use crate::frames::{FrameInputs, Frames};
use crate::kept_order::KeptOrder;
use crate::layout::LayoutBox;
use crate::paint::PaintInput;
use crate::painted::Painted;

/// `painted`'s frames for `order`'s scroll containers, each shifted by
/// its offset in `input` as `scroll::frame_shift` gives it, after an
/// update that changed `changed`, from a kept paint when `kept`, with
/// how many were worked out; whether any frame's shift or clip changed.
pub(crate) fn updated(
    painted: &mut Painted,
    order: &KeptOrder,
    (boxes, input): (&[LayoutBox], &PaintInput<'_>),
    (changed, kept): (&ChangedSlots, bool),
) -> bool {
    let reshaped = !kept || order.walks.reshaped();
    // Relaxed in a release build: the scan runs only when the frames are
    // made afresh. The scroll containers change only when paint order is
    // walked or spliced (a box that starts or stops scrolling is
    // restacked, kept_clips.rs `redo`); on a kept update `Frames::update`
    // refuses a container the frames were not made for in its place.
    if reshaped || cfg!(debug_assertions) {
        let scrolls = |slot| input.stackings.scrolls(slot);
        crate::frames_check::assert_scrollers_agree(&order.thumbs, &order.clips, &scrolls);
    }
    let depth = |slot| order.tree.depth(slot);
    let shift =
        |slot| crate::scroll::frame_shift(scroll_offset(boxes, slot, input.scroll), input.scale);
    let inputs = FrameInputs {
        clips: &order.clips,
        depth: &depth,
        shift: &shift,
    };
    if reshaped {
        return made_afresh(painted, &order.thumbs, &inputs, input.scale);
    }
    kept_updated(painted, (order, changed), &inputs, input)
}

/// `painted`'s frames made afresh for the scroll containers `thumbs`;
/// whether they differ from the kept ones.
fn made_afresh(
    painted: &mut Painted,
    thumbs: &[usize],
    inputs: &FrameInputs<'_>,
    scale: f32,
) -> bool {
    painted.frames_worked = thumbs.len();
    let made = Frames::of(thumbs, inputs, scale);
    std::mem::replace(&mut painted.frames, made) != painted.frames
}

/// `painted`'s kept frames worked out again for the scroll containers
/// whose offset moved or whose clips were redone, and the frames nested
/// in those that changed; whether any changed. A debug build compares
/// them with frames made afresh.
fn kept_updated(
    painted: &mut Painted,
    (order, changed): (&KeptOrder, &ChangedSlots),
    inputs: &FrameInputs<'_>,
    input: &PaintInput<'_>,
) -> bool {
    let starts = changed.offsets_moved.iter().chain(&order.redone).copied();
    let starts: Vec<usize> = starts
        .filter(|&slot| input.stackings.scrolls(slot))
        .collect();
    let moved;
    (moved, painted.frames_worked) = painted.frames.update(&starts, inputs);
    let fresh = || Frames::of(&order.thumbs, inputs, input.scale);
    crate::frames_check::assert_kept_frames_fresh(&painted.frames, fresh);
    moved
}

/// The scroll offset of the scroll container at `slot`, or a panic
/// naming it: placement settles every scroll container's offset
/// before paint.
fn scroll_offset(boxes: &[LayoutBox], slot: usize, scroll: &crate::scroll::Offsets) -> [f32; 2] {
    let node = boxes[slot].element_in(slot);
    node.and_then(|node| scroll.get(node)).unwrap_or_else(|| {
        panic!(
            "scroll container slot {slot} ({node:?}) has no settled offset: placement \
             settles every scroll container's (placement_walk.rs `settled_offset`); \
             reproduce with `cargo xtask test`"
        )
    })
}

#[cfg(test)]
#[path = "paint_frames_tests.rs"]
mod tests;
