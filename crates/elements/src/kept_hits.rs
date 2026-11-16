//! The hits kept across updates (IU3 3c-3g): each box step's part's
//! hits with the step's label, in painting order. Gathered from the
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
use crate::paint_labels::Label;
use crate::paint_order::Step;

/// A hit in its scroll frame: the frame, the element and its rect.
pub(crate) type FramedHit = (usize, (NodeId, BoxRect));

/// Every hit with its box step's label, by the frame it is in, each
/// frame's ordered by label; a part's hits in the order it painted them.
/// A hit test asks only the frames that can show at its point
/// (`Frames::meeting`: the scroll frames whose clip meets it, the lane
/// frames whose row meets it through the lane, section 11.2).
///
/// Exempt from the quarter-growth rule (`slot_list.rs`): the two maps
/// double, as a hash map's buckets are a power of two, so neither can
/// grow by a quarter; the growth test reports only the longest frame's
/// list, which does. The label map holds an entry a step with hits, at
/// most one a slot. Retired when hits are kept by slot (a `SlotList`)
/// rather than by label.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct KeptHits {
    by_frame: FxHashMap<usize, Vec<(Label, FramedHit)>>,
    /// The frame each label's hits are in.
    frames: FxHashMap<Label, usize>,
}

impl KeptHits {
    /// The longest frame's list of hits: name, length, capacity (the
    /// growth test, `slot_list_document_tests.rs`, reads these alone).
    #[cfg(test)]
    pub(crate) fn slot_lists(&self) -> Vec<(&'static str, usize, usize)> {
        let longest = self.by_frame.values().max_by_key(|list| list.len());
        let (len, capacity) = longest.map_or((0, 0), |list| (list.len(), list.capacity()));
        vec![("hits.by_frame", len, capacity)]
    }

    /// The hits of `scene`'s parts, in painting order.
    pub(crate) fn of(scene: &crate::SceneView<'_>) -> Self {
        let mut hits = Self::default();
        for (label, kept) in scene.labelled_parts() {
            hits.put(label, framed(kept));
        }
        hits
    }

    /// The first label, in order, that `self` and `other` hold under
    /// different frames or that one holds and the other does not, with
    /// both frames; `None` when they hold the same labels in the same
    /// frames and the same hits under each (for the kept-paint check).
    pub(crate) fn first_label_differing(&self, other: &Self) -> Option<String> {
        if self == other {
            return None;
        }
        let mut labels: Vec<Label> = self
            .frames
            .keys()
            .chain(other.frames.keys())
            .copied()
            .collect();
        labels.sort_unstable();
        labels.dedup();
        let differs = |label: &Label| self.frames.get(label) != other.frames.get(label);
        let first = labels.into_iter().find(differs);
        Some(first.map_or_else(
            || "kept hits: the same labels and frames, other hits under them".to_owned(),
            |label| {
                let (kept, fresh) = (self.frames.get(&label), other.frames.get(&label));
                format!("kept hits' label {label}: in frame {kept:?}, gathered afresh {fresh:?}")
            },
        ))
    }

    /// Puts `hits`, of the box step at `label`, in their frame's list.
    fn put(&mut self, label: Label, hits: impl Iterator<Item = FramedHit>) {
        for hit in hits {
            let list = self.by_frame.entry(hit.0).or_default();
            crate::slot_list::reserve(list, list.len() + 1);
            let at = list.partition_point(|&(labelled, _)| labelled <= label);
            list.insert(at, (label, hit));
            self.frames.insert(label, hit.0);
        }
    }

    /// Takes out the hits of the box step at `label`; them.
    pub(super) fn take(&mut self, label: Label) -> Vec<FramedHit> {
        let Some(frame) = self.frames.remove(&label) else {
            return Vec::new();
        };
        let list = self.by_frame.get_mut(&frame).unwrap_or_else(|| {
            panic!(
                "the hits of label {label} are noted in frame {frame}, which holds none \
                 (kept_hits.rs `put`); reproduce with `cargo xtask test`"
            )
        });
        let start = list.partition_point(|&(at, _)| at < label);
        let end = list.partition_point(|&(at, _)| at <= label);
        let taken = list.drain(start..end).map(|(_, hit)| hit).collect();
        if list.is_empty() {
            self.by_frame.remove(&frame);
        }
        taken
    }

    /// `self` with the hits of each of `slots`' box steps replaced by
    /// its part's in `boxes`, at the step's label in `drawn`.
    /// Returns the hits removed and added, in no order, each in its
    /// frame.
    pub(crate) fn patch(
        &mut self,
        slots: impl Iterator<Item = usize>,
        drawn: &DrawnSteps,
        boxes: &[Option<KeptBox>],
    ) -> Vec<FramedHit> {
        let mut changed = Vec::new();
        for (label, slot) in box_labels(slots, drawn) {
            changed.extend(self.take(label));
            let kept = boxes.get(slot).and_then(Option::as_ref);
            let hits: Vec<FramedHit> = kept.into_iter().flat_map(framed).collect();
            changed.extend_from_slice(&hits);
            self.put(label, hits.into_iter());
        }
        changed
    }

    /// Every hit with its frame, in painting order: O(hits log hits), for
    /// the checks.
    pub(crate) fn hits(&self) -> Vec<FramedHit> {
        let mut labelled: Vec<&(Label, FramedHit)> = self.by_frame.values().flatten().collect();
        labelled.sort_by_key(|&&(label, _)| label);
        labelled.into_iter().map(|&(_, hit)| hit).collect()
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
/// its label, ordered by label, each once.
fn box_labels(slots: impl Iterator<Item = usize>, drawn: &DrawnSteps) -> Vec<(Label, usize)> {
    let mut labels: Vec<(Label, usize)> = slots
        .filter_map(|slot| box_label(drawn, slot).map(|label| (label, slot)))
        .collect();
    labels.sort_unstable();
    labels.dedup();
    labels
}

/// The label of `slot`'s box step in paint order, if it has one.
fn box_label(drawn: &DrawnSteps, slot: usize) -> Option<Label> {
    let mut steps = drawn.labels_of(slot);
    steps.find_map(|(label, step)| matches!(step, Step::Box(_)).then_some(label))
}

#[path = "kept_hits_query.rs"]
mod query;
#[path = "kept_hits_splice.rs"]
mod splice;

#[cfg(test)]
#[path = "kept_hits_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "kept_hits_lane_tests.rs"]
mod lane_tests;
