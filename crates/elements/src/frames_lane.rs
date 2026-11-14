//! Lane frames (`docs/plans/splice-layout.md`, section 11): a framed
//! lane child is a frame of its own, shifted from the frame its
//! container's content is in by the container's origin there and the
//! child's offset in the lane, and clipped as the container clips its
//! content. Resolved when first read and memoised until the frames are
//! given new lanes, once per update: an insert changes every later
//! child's shift, and only the frames read (drawn, hit) are worked out.

use std::cell::RefCell;

use rustc_hash::FxHashMap;

use super::{FrameAt, Frames, meet};
use vapors_scene::Rect;

use crate::layout::{BoxRect, SharedLanes};
use crate::paint::snapped;

/// What a framed lane's children's frames are worked out from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct LaneBase {
    /// The frame the container's content is in.
    pub(crate) parent: usize,
    /// The container's border box top in that frame, CSS pixels.
    pub(crate) origin: f32,
    /// The clip the container gives its content in that frame; `None`
    /// when it scrolls, its scroll frame clipping already, or clips
    /// nothing.
    pub(crate) inner: Option<BoxRect>,
}

/// The lanes the frames read and each framed container's base.
#[derive(Debug, Default)]
pub(crate) struct LaneFrames {
    shared: Option<SharedLanes>,
    bases: FxHashMap<usize, LaneBase>,
    /// The framed containers whose content is in each frame.
    by_parent: FxHashMap<usize, Vec<usize>>,
    memo: RefCell<FxHashMap<usize, FrameAt>>,
}

/// How far each framed lane's children's ink reaches past their boxes,
/// the most seen: a history, which a paint made afresh does not have,
/// so frames compare equal whatever it holds. It only widens culling,
/// which the debug build checks against testing every step
/// (`kept_batches_check.rs`).
#[derive(Debug, Default)]
pub(crate) struct LaneInk(FxHashMap<usize, [f32; 2]>);

impl PartialEq for LaneInk {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

/// The bases alone: the lanes are the boxes', and the memo is what was
/// read.
impl PartialEq for LaneFrames {
    fn eq(&self, other: &Self) -> bool {
        self.bases == other.bases
    }
}

impl Frames {
    /// The lane frames from now on: those of the children of the
    /// containers in `bases`, from `shared`.
    pub(crate) fn set_lanes(&mut self, shared: SharedLanes, bases: FxHashMap<usize, LaneBase>) {
        let mut by_parent: FxHashMap<usize, Vec<usize>> = FxHashMap::default();
        for (&container, base) in &bases {
            by_parent.entry(base.parent).or_default().push(container);
        }
        self.lanes = LaneFrames {
            shared: Some(shared),
            bases,
            by_parent,
            memo: RefCell::default(),
        };
    }

    /// The lane frames in frame `frame` that can show in `view`, device
    /// pixels on the page: the children of each framed lane in it whose
    /// box or overflow meets the view as it falls in the frame, a device
    /// pixel wider each way (`LaneTree::between`, O(log children +
    /// shown) per lane).
    pub(crate) fn lane_frames_meeting(&self, frame: usize, view: Rect) -> Vec<usize> {
        let (Some(containers), Some(shared)) =
            (self.lanes.by_parent.get(&frame), &self.lanes.shared)
        else {
            return Vec::new();
        };
        let in_frame = self.view_in(frame, view);
        let scale = f64::from(self.scale);
        let lanes = shared.borrow();
        let meeting = |container: &usize| {
            let origin = f64::from(self.lanes.bases[container].origin);
            let [above, below] = self.lane_ink.0.get(container).copied().unwrap_or_default();
            let from = (f64::from(in_frame.top) - 1.0) / scale - origin - f64::from(below);
            let to = (f64::from(in_frame.bottom) + 1.0) / scale - origin + f64::from(above);
            lanes.children_between(*container, from, to)
        };
        containers.iter().flat_map(meeting).collect()
    }

    /// Notes that ink in a child of the framed lane container
    /// `container` reaches `over` past its border box, above and below,
    /// CSS pixels: culling widens by the most noted (a box shadow, a
    /// glyph's overhang), which can only grow until the frames are made
    /// afresh, so it stays sound when the child that set it is gone.
    pub(crate) fn note_lane_ink(&mut self, container: usize, over: [f32; 2]) {
        let most = self.lane_ink.0.entry(container).or_default();
        *most = [most[0].max(over[0]), most[1].max(over[1])];
    }

    /// Lane frame `frame`, when `frame` is a framed lane child.
    #[allow(clippy::cast_possible_truncation)] // device px, whole, within f32
    pub(super) fn lane_at(&self, frame: usize) -> Option<FrameAt> {
        if let Some(&at) = self.lanes.memo.borrow().get(&frame) {
            return Some(at);
        }
        let (base, offset) = self.lane_place(frame)?;
        let outer = self.at(base.parent);
        let down = (f64::from(base.origin) + offset) * f64::from(self.scale);
        let clip = match base.inner {
            Some(inner) => meet(Some(self.css_rect(outer, inner)), outer.clip),
            None => outer.clip,
        };
        let at = FrameAt {
            shift: [outer.shift[0], outer.shift[1] + down as f32],
            clip,
            device_clip: clip.map(|clip| snapped(clip, self.scale)),
        };
        self.lanes.memo.borrow_mut().insert(frame, at);
        Some(at)
    }

    /// The base of the framed lane `frame` is a child of and the
    /// child's offset in it, CSS pixels.
    fn lane_place(&self, frame: usize) -> Option<(LaneBase, f64)> {
        let lanes = self.lanes.shared.as_ref()?.borrow();
        let container = lanes.container_of(frame)?;
        let base = *self.lanes.bases.get(&container)?;
        Some((base, lanes.frame_offset(container, frame)?))
    }
}
