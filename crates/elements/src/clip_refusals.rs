//! The boxes a rounded clip refuses (flat-scene F3.2b,
//! `docs/plans/flat-scene-f3.md`): worked out with each box's clips
//! (`kept_clips.rs` keeps them by slot), and the update refused once,
//! after paint, naming the first in tree order and counting the others,
//! as a clip chain past `MAX_CLIP_CHAIN` is (`paint_chain_refusals.rs`).

use crate::clip::BoxClip;
use crate::document::Document;
use crate::error::ElementError;
use crate::layout::{BoxRect, LayoutBox, Placed};
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
    /// A scroll container that a rounded clip above it reaches: its
    /// content's clip is its frame's, which is not rounded while a rounded
    /// scroller is deferred (`docs/plans/frame-clips.md`).
    Frame,
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
    } else if frames && rounded(inner) {
        Some(RoundedRefusal::Frame)
    } else {
        None
    }
}

/// What a rounded frame, `round`, refuses of box `layout_box` painted in
/// it: an img or a gpu-surface, which draw under the scissor; and a
/// frame of its own (`frames`), its content's region on the page
/// `region`, not held clear of the frame's corners (`clip_rounded.rs`
/// `holds`). The first frame entries chain to their outer frame's lifts
/// the second (`docs/plans/frame-clips.md`, the follow-up).
pub(crate) fn in_frame(
    layout_box: &LayoutBox,
    round: crate::frames::FrameRound,
    (frames, region): (bool, BoxRect),
) -> Option<RoundedRefusal> {
    if layout_box.image.is_some() {
        Some(RoundedRefusal::Image)
    } else if layout_box.surface.is_some() {
        Some(RoundedRefusal::Surface)
    } else if frames && !crate::clip_rounded::holds(round.rect, round.radii, region) {
        Some(RoundedRefusal::NestedFrame {
            rounded_by: round.by,
        })
    } else {
        None
    }
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

impl Document {
    /// One refusal for `refused` (slot, why), naming the first in tree
    /// order.
    ///
    /// # Errors
    /// Any box in `refused`.
    pub(crate) fn refuse_rounded(
        &self,
        refused: &[(usize, RoundedRefusal)],
        placed: &Placed,
    ) -> Result<(), Vec<ElementError>> {
        if refused.is_empty() {
            return Ok(());
        }
        let order = crate::paint_cause::tree_order(placed);
        let first = order
            .iter()
            .find_map(|&slot| refused.iter().find(|(at, _)| *at == slot));
        let &(slot, why) = first.unwrap_or_else(|| {
            panic!(
                "no box a rounded clip refuses is in the tree's order ({refused:?}): `Clips` \
                 keeps refusals only for kept slots, clearing a freed slot's (kept_clips.rs); \
                 reproduce with `cargo xtask test`"
            )
        });
        let describe = |slot: usize| {
            placed.slots.key(slot).map_or_else(
                || "a box with no element".to_owned(),
                |key| crate::check::describe(&self.arena, key.node()),
            )
        };
        let mut element = describe(slot);
        if let RoundedRefusal::NestedFrame { rounded_by } = why {
            element = format!(
                "{element} (in the frame rounded by {})",
                describe(rounded_by)
            );
        }
        let others = refused.len() - 1;
        Err(vec![ElementError::Refused(message(&element, why, others))])
    }
}

fn message(element: &str, why: RoundedRefusal, others: usize) -> String {
    let (what, fix) = match why {
        RoundedRefusal::Overlap => (
            "its clip and a rounded clip it is under (overflow: hidden or clip with a \
             border-radius) overlap without either holding the other, which needs clip chains",
            "keep the clipping box inside the rounded box's padding box clear of its corners, or \
             let it hold the rounded box whole",
        ),
        RoundedRefusal::FieldText => (
            "is a text field and its content box, where its text and placeholder paint, crosses \
             the corner of a rounded clip it is under (overflow: hidden or clip with a \
             border-radius), which would cut its placeholder square",
            "move the field clear of the rounded box's corners, or give it padding past them",
        ),
        RoundedRefusal::Image => (
            "is an img under a rounded clip, and an image draws under a square scissor until \
             F3.1b",
            "remove the border-radius or the clip from the box above it",
        ),
        RoundedRefusal::Surface => (
            "is a gpu-surface under a rounded clip, and a surface draws under a square scissor \
             until F3.1b",
            "remove the border-radius or the clip from the box above it",
        ),
        RoundedRefusal::Frame => (
            "scrolls under a rounded clip that it holds, and a scroller's clip is not rounded \
             while rounded scrollers are deferred",
            "fit this box inside the rounded box's padding box clear of its corners",
        ),
        RoundedRefusal::NestedFrame { .. } => (
            "scrolls, or frames a lane's rows, inside a frame with a rounded clip, crossing its \
             corners, and a frame's clip takes an outer frame's corners only once frame entries \
             chain to their outer frame's (docs/plans/frame-clips.md, the follow-up)",
            "keep this box inside the rounded box's padding box clear of its corners",
        ),
    };
    let also = match others {
        0 => String::new(),
        1 => "; 1 other box is refused too".to_owned(),
        n => format!("; {n} other boxes are refused too"),
    };
    format!(
        "{element}: {what} (docs/plans/flat-scene-f3.md F3.2b){also}; fix: {fix}; reproduce: \
         update the document, as `cargo test -p vapors-elements --test tests rounded_clip` does"
    )
}
