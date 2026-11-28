//! The boxes a rounded clip refuses (flat-scene F3.2b,
//! `docs/plans/flat-scene-f3.md`): worked out with each box's clips
//! (`kept_clips.rs` keeps them by slot), and, for the boxes in a rounded
//! lane frame, against its corners after paint. Reporting them is
//! `clip_refusals_report.rs`.

use crate::clip::BoxClip;
use crate::layout::{BoxRect, LayoutBox};
use crate::paint::PaintInput;

/// Why a rounded clip refuses a box.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum RoundedRefusal {
    /// Its clip and the clip it is under overlap without either holding
    /// the other, so neither is the clip (Tom, 2026-11-27).
    Overlap,
    /// A text field whose content box, where its text and placeholder
    /// paint, overlaps a rounded clip it paints under without being held
    /// by it: its text would be cut square.
    FieldText,
    /// An `img` under a rounded clip: an image draws under the scissor,
    /// which cuts square, until F3.1b moves images onto the clip table.
    Image,
    /// A `gpu-surface` under a rounded clip, as an image.
    Surface,
    /// An svg drawing shapes under a rounded clip: a path is tessellated
    /// into a mesh, which draws under the scissor, as an image.
    Svg,
    /// A scroll container that a rounded clip above it reaches: its
    /// content's clip is its frame's, which is not rounded while a rounded
    /// scroller is deferred (`docs/plans/frame-clips.md`).
    Frame,
    /// A lane container whose children are frames and which does not
    /// clip, in a frame rounded by box `rounded_by`: its rows reach past
    /// its box, wherever it is.
    UnclippedLane {
        /// The slot of the box whose rounded clip the outer frame has.
        rounded_by: usize,
    },
    /// A scroll container, or a lane container whose children are frames,
    /// in a frame rounded by box `rounded_by` and not held clear of its
    /// corners: its frame's clip would not take the corners until frame
    /// entries chain to their outer frame's (`docs/plans/frame-clips.md`,
    /// the follow-up).
    NestedFrame {
        /// The slot of the box whose rounded clip the outer frame has.
        rounded_by: usize,
    },
}

/// What a rounded clip refuses of box `layout_box`, under `own`, giving
/// its content `inner`: `overlaps` when the two overlap as they must not
/// (`clip_rounded.rs`), `field_text` when it is a text field whose
/// content box overlaps a rounded clip so, `frames` when its content is
/// a frame of its own.
pub(crate) fn of(
    layout_box: &LayoutBox,
    (own, inner): (Option<BoxClip>, Option<BoxClip>),
    (overlaps, field_text): (bool, bool),
    frames: bool,
) -> Option<RoundedRefusal> {
    let rounded = |clip: Option<BoxClip>| clip.is_some_and(|clip| clip.rounded_by().is_some());
    if overlaps {
        Some(RoundedRefusal::Overlap)
    } else if field_text {
        Some(RoundedRefusal::FieldText)
    } else if rounded(own) && layout_box.image.is_some() {
        Some(RoundedRefusal::Image)
    } else if rounded(own) && layout_box.surface.is_some() {
        Some(RoundedRefusal::Surface)
    } else if rounded(own) && layout_box.svg_shapes.is_some() {
        Some(RoundedRefusal::Svg)
    } else if frames && rounded(inner) {
        Some(RoundedRefusal::Frame)
    } else {
        None
    }
}

/// Whether box `slot` rounds what it holds: the clip it gives its content,
/// `clip`, is its own rounded one. One answer for whether such a box is
/// checked against a rounded frame's corners (`in_round_frame`) and for
/// the reason it is refused (`nested_frames`): a box whose clip is a
/// square one it holds, `meet` having kept that, is neither.
pub(crate) fn rounds_itself(clip: Option<BoxClip>, slot: usize) -> bool {
    clip.and_then(|clip| clip.rounded_by()) == Some(slot)
}

/// What a lane frame rounded by box `by` refuses of box `layout_box` in
/// it, decided with its clips, not its place: an img, a gpu-surface and
/// an svg drawing shapes, which draw under the scissor; and a framed
/// lane that does not clip (`is_frame`, `clips`), whose rows' reach is
/// not its box. A frame that clips, and a box rounding what it holds
/// (`rounds`, `rounds_itself`), are returned to be checked against the
/// corners after paint (`nested_frames`), as rows move with none of their
/// boxes painted again.
pub(crate) fn in_round_frame(
    layout_box: &LayoutBox,
    by: usize,
    (is_frame, clips, rounds): (bool, bool, bool),
) -> (Option<RoundedRefusal>, Option<usize>) {
    let refused = if layout_box.image.is_some() {
        Some(RoundedRefusal::Image)
    } else if layout_box.surface.is_some() {
        Some(RoundedRefusal::Surface)
    } else if layout_box.svg_shapes.is_some() {
        Some(RoundedRefusal::Svg)
    } else if is_frame && !clips {
        Some(RoundedRefusal::UnclippedLane { rounded_by: by })
    } else {
        None
    };
    // a box rounding its own content is checked too: the painter chains
    // its rounding under the frame's, the page view holds one
    (
        refused,
        (refused.is_none() && (is_frame || rounds)).then_some(by),
    )
}

/// The frames, and the boxes rounding what they hold, in a rounded lane
/// frame not held clear of its corners (`clip_rounded.rs` `holds`), each
/// refused, as `Overlap` for a box rounding itself and `NestedFrame` for
/// a frame: their content clips on the page against the frame's rounded
/// clip, every update, one check a frame (`Clips::nested`). The first
/// frame entries chain to their outer frame's lifts this
/// (`docs/plans/frame-clips.md`, the follow-up).
pub(crate) fn nested_frames(
    clips: &crate::kept_clips::Clips,
    frames: &crate::frames::Frames,
) -> Vec<(usize, RoundedRefusal)> {
    let crosses = |&(slot, by): &(usize, usize)| {
        let frame = clips.frame(slot);
        let region = frames.on_page(frame, nested_clip(clips, slot));
        let round = frame_round(frames, (slot, by, frame));
        let held = crate::clip_rounded::holds(round.rect, round.radii, region);
        let why = if rounds_itself(clips.inner(slot), slot) {
            RoundedRefusal::Overlap
        } else {
            RoundedRefusal::NestedFrame { rounded_by: by }
        };
        (!held).then_some((slot, why))
    };
    clips.nested().iter().filter_map(crosses).collect()
}

/// The rounded clip of `frame`, which nested box `slot` is in, rounded by
/// box `by`.
///
/// # Panics
///
/// When the frame has none: `Clips::nested` and the frames must agree.
fn frame_round(
    frames: &crate::frames::Frames,
    (slot, by, frame): (usize, usize, usize),
) -> crate::frames::FrameRound {
    frames
        .get(frame)
        .and_then(|at| at.rounded)
        .unwrap_or_else(|| {
            panic!(
                "box slot {slot} is kept nested in a lane frame rounded by box slot {by} \
             (`Clips::nested`), and its frame {frame} has no rounded clip: the two must agree \
             (kept_clips_round.rs, frames_lane_prime.rs); reproduce with `cargo xtask test`"
            )
        })
}

/// The clip nested box `slot` gives its content, in its frame.
///
/// # Panics
///
/// When it gives none: only a box that clips is kept nested, a frame or
/// a box rounding what it holds.
fn nested_clip(clips: &crate::kept_clips::Clips, slot: usize) -> crate::layout::BoxRect {
    let clip = clips.inner(slot).unwrap_or_else(|| {
        panic!(
            "box slot {slot}, kept nested in a rounded lane frame as a frame or a box rounding \
             what it holds, gives no clip \
             (clip_refusals.rs `in_round_frame`); reproduce with `cargo xtask test`"
        )
    });
    clip.rect
}

/// Whether box `slot` of `boxes` at `rects` is a text field whose content
/// box, where its text and placeholder paint, overlaps a rounded clip of
/// `clips`, the clip it is under and the one it gives its content,
/// without being held by it (`clip_rounded.rs`). Its value's run paints
/// under the clip it gives, which rounds it; its placeholder is cut
/// square to the content box (`placeholder_paint.rs`), so a field with
/// no placeholder showing is refused too, which is more than needed.
pub(crate) fn field_text_overlaps(
    (boxes, rects, input): (&[LayoutBox], &[BoxRect], &PaintInput<'_>),
    slot: usize,
    (own, inner): (Option<BoxClip>, Option<BoxClip>),
) -> bool {
    if !boxes[slot].text_field {
        return false;
    }
    let content = BoxClip::square(crate::clip::content_box(&boxes[slot], rects[slot]));
    let radii_of = |at| crate::clip::radii(boxes, rects, at, input);
    let crosses = |clip: Option<BoxClip>| {
        let rounded = clip.filter(|clip| clip.rounded_by().is_some());
        rounded.is_some_and(|clip| {
            let met = crate::clip_rounded::meet(Some(clip), content, &radii_of);
            matches!(met, crate::clip_rounded::Met::Overlap(_))
        })
    };
    // one check when the field does not clip, and gives what it is under
    crosses(own) || (inner != own && crosses(inner))
}
