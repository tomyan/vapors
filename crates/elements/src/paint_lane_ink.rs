//! How far painted ink reaches past a framed lane child's border box
//! (`frames_lane.rs` `note_lane_ink`): culling finds a lane's children
//! that can show from their boxes and overflow, and ink past both (a
//! box shadow, a glyph's overhang) would be missed without it. Noted for
//! the lane frames whose parts an update painted, after the extents
//! index is patched for them.

use rustc_hash::FxHashSet;

use crate::paint::PaintInput;
use crate::painted::Painted;

/// Notes the ink of the lane frames this update painted: every one when
/// paint order was `reshaped`, else those holding the `patched` slots.
pub(crate) fn note_painted(
    painted: &mut Painted,
    input: &PaintInput<'_>,
    (reshaped, patched): (bool, &[usize]),
) {
    if reshaped {
        note(painted, input, 0..painted.kept_boxes.len());
    } else {
        note(painted, input, patched.iter().copied());
    }
}

/// Notes the ink of the lane frames holding any of `slots`' parts.
fn note(painted: &mut Painted, input: &PaintInput<'_>, slots: impl Iterator<Item = usize>) {
    for frame in lane_frames_of(painted, input, slots) {
        let Some([top, bottom]) = painted.extents.frame_span(frame) else {
            continue;
        };
        let scale = input.scale;
        note_out(painted, input, frame, [top / scale, bottom / scale]);
    }
}

/// The lane frames holding any of `slots`' parts, each once.
fn lane_frames_of(
    painted: &Painted,
    input: &PaintInput<'_>,
    slots: impl Iterator<Item = usize>,
) -> FxHashSet<usize> {
    let framed = |frame: &usize| input.placement.framed.get(*frame).copied().unwrap_or(false);
    let frame_of = |slot: usize| {
        let part = painted.kept_boxes.get(slot)?.as_ref()?;
        Some(part.from.frame).filter(framed)
    };
    slots.filter_map(frame_of).collect()
}

/// Notes how far `span`, CSS pixels in lane frame `frame`, reaches past
/// the frame's row, on the row's lane; then, carried into the frame the
/// lane's container is in, on each lane that container is nested in, so
/// an outer lane finds a row whose inner lane's ink reaches past it.
fn note_out(painted: &mut Painted, input: &PaintInput<'_>, frame: usize, span: [f32; 2]) {
    let placement = input.placement;
    let (mut frame, mut span) = (frame, span);
    loop {
        let (rect, container) = (placement.rects[frame], placement.up[frame]);
        let over = [rect.y - span[0], span[1] - (rect.y + rect.height)];
        painted
            .frames
            .note_lane_ink(container, over.map(|v| v.max(0.0)));
        let Some(outer) = outer_lane_frame(painted, input, container) else {
            return;
        };
        let into = placement.rects[container].y + lane_offset(input, container, frame);
        (frame, span) = (outer, span.map(|v| v + into));
    }
}

/// The lane frame the parts of `container` are in, if it is one: the
/// container itself when it is a row of an outer lane.
fn outer_lane_frame(painted: &Painted, input: &PaintInput<'_>, container: usize) -> Option<usize> {
    let part = painted.kept_boxes.get(container)?.as_ref()?;
    let frame = part.from.frame;
    input
        .placement
        .framed
        .get(frame)
        .copied()
        .unwrap_or(false)
        .then_some(frame)
}

/// Where lane frame `frame` starts in the border box of `container`.
fn lane_offset(input: &PaintInput<'_>, container: usize, frame: usize) -> f32 {
    let offset = input.lanes.borrow().frame_offset(container, frame);
    let offset = offset.unwrap_or_else(|| {
        panic!(
            "lane frame {frame} is framed but the lane of container {container} does not \
             hold it (paint_lane_ink.rs); reproduce with `cargo xtask test`"
        )
    });
    crate::layout::narrow(offset)
}
