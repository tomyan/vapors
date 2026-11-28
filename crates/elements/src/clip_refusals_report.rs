//! Reporting what a rounded clip refuses (`clip_refusals.rs` decides it):
//! the refusals one a slot, and the update refused once, after paint,
//! naming the first in tree order and counting the others, as a clip
//! chain past `MAX_CLIP_CHAIN` is (`paint_chain_refusals.rs`).

use crate::clip_refusals::RoundedRefusal;
use crate::document::Document;
use crate::error::ElementError;
use crate::layout::Placed;

/// `clips`' refusals and the rounded frames' `frames`, one a slot, in
/// slot order: a box both refuse is counted once, under the reason that
/// comes first in `RoundedRefusal`'s order (`Frame` before `NestedFrame`,
/// so a scroller is told about its own rounded clip first).
pub(crate) fn by_slot(
    clips: Vec<(usize, RoundedRefusal)>,
    frames: Vec<(usize, RoundedRefusal)>,
) -> Vec<(usize, RoundedRefusal)> {
    let mut refused = [clips, frames].concat();
    refused.sort_unstable();
    refused.dedup_by_key(|&mut (slot, _)| slot);
    refused
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
        if let RoundedRefusal::NestedFrame { rounded_by }
        | RoundedRefusal::UnclippedLane { rounded_by } = why
        {
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
    let (what, fix) = wording(why);
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

/// What `why` says went wrong, and its fix.
fn wording(why: RoundedRefusal) -> (&'static str, &'static str) {
    match why {
        RoundedRefusal::Overlap => (
            "its clip and a rounded clip it is under (overflow: hidden or clip with a \
             border-radius) overlap without either holding the other, which needs clip chains",
            "keep the clipping box inside the rounded box's padding box clear of its corners, or, \
             outside a framed lane's rows, let it hold the rounded box whole",
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
        RoundedRefusal::Svg => (
            "is an svg drawing shapes under a rounded clip, and a path draws as a mesh under a \
             square scissor until F3.1b",
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
        RoundedRefusal::UnclippedLane { .. } => (
            "frames a lane's rows without clipping them, inside a frame with a rounded clip, so \
             its rows can reach the corners, and a frame's clip takes an outer frame's corners \
             only once frame entries chain to their outer frame's (docs/plans/frame-clips.md, \
             the follow-up)",
            "give this box overflow: hidden or clip, and keep it clear of the rounded box's \
             corners",
        ),
        RoundedRefusal::NestedFrame { .. } => (
            "scrolls, or frames a lane's rows, inside a frame with a rounded clip, crossing its \
             corners, and a frame's clip takes an outer frame's corners only once frame entries \
             chain to their outer frame's (docs/plans/frame-clips.md, the follow-up)",
            "keep this box inside the rounded box's padding box clear of its corners",
        ),
    }
}
