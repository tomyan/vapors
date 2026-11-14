//! The hits kept across updates (IU3 3c-3g): each box step's part's
//! hits with the step's place, in painting order. Gathered from the
//! scene view on the first hit test after paint order is walked again;
//! else patched for the slots an update paints again or frees
//! (`paint_partial.rs`), as the drawing steps are (`drawn_steps.rs`). A
//! debug build compares it with a fresh gather on every update and
//! names the first differing hit (`kept_paint_check.rs`). The patch
//! records the hits it removed and added (`HitsChanged`), which decides
//! whether the hover is tested again after the update. Each hit is kept
//! in its part's scroll frame and put on the page as it is tested
//! (`frames.rs`).

use rustc_hash::FxHashMap;
use vapors_dom::NodeId;

use crate::drawn_steps::DrawnSteps;
use crate::frames::Frames;
use crate::kept_box::KeptBox;
use crate::layout::BoxRect;
use crate::paint_order::Step;

/// A hit in its scroll frame: the frame, the element and its rect.
pub(crate) type FramedHit = (usize, (NodeId, BoxRect));

/// Every hit with its box step's place, by the frame it is in, each
/// frame's ordered by place; a part's hits in the order it painted them.
/// A hit test asks only the frames that can show at its point
/// (`Frames::meeting`: the scroll frames whose clip meets it, the lane
/// frames whose row meets it through the lane, section 11.2).
#[derive(Debug, Default, PartialEq)]
pub(crate) struct KeptHits {
    by_frame: FxHashMap<usize, Vec<(usize, FramedHit)>>,
    /// The frame each place's hits are in.
    frames: FxHashMap<usize, usize>,
}

impl KeptHits {
    /// The hits of `scene`'s parts, in painting order.
    pub(crate) fn of(scene: &crate::SceneView<'_>) -> Self {
        let mut hits = Self::default();
        for (place, kept) in scene.placed_parts() {
            hits.put(place, framed(kept));
        }
        hits
    }

    /// Puts `hits`, of the box step at `place`, in their frame's list.
    fn put(&mut self, place: usize, hits: impl Iterator<Item = FramedHit>) {
        for hit in hits {
            let list = self.by_frame.entry(hit.0).or_default();
            let at = list.partition_point(|&(placed, _)| placed <= place);
            list.insert(at, (place, hit));
            self.frames.insert(place, hit.0);
        }
    }

    /// Takes out the hits of the box step at `place`; them.
    fn take(&mut self, place: usize) -> Vec<FramedHit> {
        let Some(frame) = self.frames.remove(&place) else {
            return Vec::new();
        };
        let list = self.by_frame.get_mut(&frame).unwrap_or_else(|| {
            panic!(
                "the hits of place {place} are noted in frame {frame}, which holds none \
                 (kept_hits.rs `put`); reproduce with `cargo xtask test`"
            )
        });
        let start = list.partition_point(|&(at, _)| at < place);
        let end = list.partition_point(|&(at, _)| at <= place);
        let taken = list.drain(start..end).map(|(_, hit)| hit).collect();
        if list.is_empty() {
            self.by_frame.remove(&frame);
        }
        taken
    }

    /// `self` with the hits of each of `slots`' box steps replaced by
    /// its part's in `boxes`, at the step's place in `drawn`.
    /// Returns the hits removed and added, in no order, each in its
    /// frame.
    pub(crate) fn patch(
        &mut self,
        slots: impl Iterator<Item = usize>,
        drawn: &DrawnSteps,
        boxes: &[Option<KeptBox>],
    ) -> Vec<FramedHit> {
        let mut changed = Vec::new();
        for (place, slot) in box_places(slots, drawn) {
            changed.extend(self.take(place));
            let kept = boxes.get(slot).and_then(Option::as_ref);
            let hits: Vec<FramedHit> = kept.into_iter().flat_map(framed).collect();
            changed.extend_from_slice(&hits);
            self.put(place, hits.into_iter());
        }
        changed
    }

    /// Every hit with its frame, in painting order: O(hits log hits), for
    /// the checks.
    pub(crate) fn hits(&self) -> Vec<FramedHit> {
        let mut placed: Vec<&(usize, FramedHit)> = self.by_frame.values().flatten().collect();
        placed.sort_by_key(|&&(place, _)| place);
        placed.into_iter().map(|&(_, hit)| hit).collect()
    }
}

/// `kept`'s own hits, each with its frame.
fn framed(kept: &KeptBox) -> impl Iterator<Item = FramedHit> + '_ {
    kept.own.hits.iter().map(|&hit| (kept.from.frame, hit))
}

/// Whether `rect` holds `[x, y]`: its left and top edges, not its right
/// and bottom.
fn holds(rect: &BoxRect, [x, y]: [f32; 2]) -> bool {
    x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height
}

/// The hits an update removed or added, when known: a hit test can
/// answer differently from before the update only at a point one of
/// them holds, so the hover after an update is tested again only then
/// (`events_hit.rs`, M11).
#[derive(Debug, Default, PartialEq)]
pub(crate) enum HitsChanged {
    /// The hits were gathered afresh, or none are kept yet: any may
    /// differ.
    #[default]
    Unknown,
    /// The hits removed and added, in no order, on the page.
    Known(Vec<(NodeId, BoxRect)>),
}

impl HitsChanged {
    /// The `changed` hits, put on the page by `frames`; unknown when a
    /// changed hit's frame is gone (a lane child taken out with its
    /// hits), which is the safe answer.
    pub(crate) fn on_page(changed: Vec<FramedHit>, frames: &Frames) -> Self {
        let hits = changed.into_iter();
        let on_page =
            hits.map(|(frame, (node, rect))| Some((node, frames.hit_if_held(frame, rect)?)));
        on_page
            .collect::<Option<_>>()
            .map_or(Self::Unknown, Self::Known)
    }

    /// Whether a hit test at `point` may name another element than
    /// `hovered`: when unknown, and when a changed hit holds `point` or
    /// is `hovered`'s.
    pub(crate) fn may_move_hover(&self, point: [f32; 2], hovered: Option<NodeId>) -> bool {
        match self {
            Self::Unknown => true,
            Self::Known(hits) => hits
                .iter()
                .any(|(node, rect)| Some(*node) == hovered || holds(rect, point)),
        }
    }
}

/// Each of `slots` whose box step is in `drawn`'s paint order, after
/// its place, ordered by place, each once.
fn box_places(slots: impl Iterator<Item = usize>, drawn: &DrawnSteps) -> Vec<(usize, usize)> {
    let mut places: Vec<(usize, usize)> = slots
        .filter_map(|slot| box_place(drawn, slot).map(|place| (place, slot)))
        .collect();
    places.sort_unstable();
    places.dedup();
    places
}

/// The place of `slot`'s box step in paint order, if it has one.
fn box_place(drawn: &DrawnSteps, slot: usize) -> Option<usize> {
    let mut steps = drawn.places_of(slot);
    steps.find_map(|(place, step)| matches!(step, Step::Box(_)).then_some(place))
}

#[path = "kept_hits_query.rs"]
mod query;

#[cfg(test)]
#[path = "kept_hits_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "kept_hits_lane_tests.rs"]
mod lane_tests;
