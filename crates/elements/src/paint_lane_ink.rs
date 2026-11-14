//! How far painted ink reaches past a framed lane child's border box
//! (`frames_lane.rs` `note_lane_ink`): culling finds a lane's children
//! that can show from their boxes and overflow, and ink past both (a
//! box shadow, a glyph's overhang) would be missed without it. Noted for
//! the lane frames whose parts an update painted, after the extents
//! index is patched for them.

use rustc_hash::FxHashSet;

use crate::paint::PaintInput;
use crate::painted::Painted;

/// Notes the ink of the lane frames holding any of `slots`' parts.
pub(crate) fn note(
    painted: &mut Painted,
    input: &PaintInput<'_>,
    slots: impl Iterator<Item = usize>,
) {
    let placement = input.placement;
    let mut frames = FxHashSet::default();
    for slot in slots {
        let frame = painted
            .kept_boxes
            .get(slot)
            .and_then(Option::as_ref)
            .map(|part| part.from.frame);
        frames.extend(frame.filter(|&frame| placement.framed.get(frame).copied().unwrap_or(false)));
    }
    for frame in frames {
        let Some([top, bottom]) = painted.extents.frame_span(frame) else {
            continue;
        };
        let (rect, scale) = (placement.rects[frame], input.scale);
        let over = [
            rect.y - top / scale,
            bottom / scale - (rect.y + rect.height),
        ];
        painted
            .frames
            .note_lane_ink(placement.up[frame], over.map(|v| v.max(0.0)));
    }
}
