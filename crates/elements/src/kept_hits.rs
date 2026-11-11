//! The hits kept across updates (IU3 3c-3g): each box step's part's
//! hits with the step's place, in painting order. Gathered from the
//! scene view on the first hit test after paint order is walked again;
//! else patched for the slots an update paints again or frees
//! (`paint_partial.rs`), as the drawing steps are (`drawn_steps.rs`). A
//! debug build compares it with a fresh gather on every update and
//! names the first differing hit (`kept_paint_check.rs`).

use vapors_dom::NodeId;

use crate::drawn_steps::DrawnSteps;
use crate::kept_box::KeptBox;
use crate::layout::BoxRect;
use crate::paint_order::Step;

/// Every hit with its box step's place, ordered by place; a part's hits
/// in the order it painted them.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct KeptHits {
    placed: Vec<(usize, (NodeId, BoxRect))>,
}

impl KeptHits {
    /// The hits of `scene`'s parts, in painting order.
    pub(crate) fn of(scene: &crate::SceneView<'_>) -> Self {
        let placed = scene
            .placed_parts()
            .flat_map(|(place, part)| part.hits.iter().map(move |&hit| (place, hit)));
        Self {
            placed: placed.collect(),
        }
    }

    /// `self` with the hits of each of `slots`' box steps replaced by
    /// its part's in `boxes`, at the step's place in `drawn`: one
    /// splice per place, from the last, so the list stays ordered by
    /// place as a fresh gather is.
    pub(crate) fn patch(
        &mut self,
        slots: impl Iterator<Item = usize>,
        drawn: &DrawnSteps,
        boxes: &[Option<KeptBox>],
    ) {
        for (place, slot) in box_places(slots, drawn).into_iter().rev() {
            let start = self.placed.partition_point(|&(at, _)| at < place);
            let end = self.placed.partition_point(|&(at, _)| at <= place);
            let kept = boxes.get(slot).and_then(Option::as_ref);
            let hits = kept.map_or(&[][..], |kept| &kept.own.hits);
            let placed = hits.iter().map(|&hit| (place, hit));
            self.placed.splice(start..end, placed);
        }
    }

    /// Every hit, in painting order.
    pub(crate) fn hits(&self) -> impl DoubleEndedIterator<Item = &(NodeId, BoxRect)> {
        self.placed.iter().map(|(_, hit)| hit)
    }

    /// The last hit whose rect holds `[x, y]`: its left and top edges,
    /// not its right and bottom.
    pub(crate) fn hit(&self, [x, y]: [f32; 2]) -> Option<NodeId> {
        self.hits()
            .rev()
            .find(|(_, r)| x >= r.x && x < r.x + r.width && y >= r.y && y < r.y + r.height)
            .map(|(node, _)| *node)
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

#[cfg(test)]
#[path = "kept_hits_tests.rs"]
mod tests;
