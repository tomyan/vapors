//! A paint's scene as a view over its kept paint order and each slot's
//! kept part (IU3 3c-3d2, `docs/plans/iu3.md` "The scene in stable
//! slots"): the items are each step's part's, walked in order, so no
//! update copies a kept part into a flat list. Hit testing reads the
//! parts' hits in the same order (`Painted::hit`).

use vapors_dom::NodeId;
use vapors_scene::{Batched, Item, Rect, Scene};

use crate::kept_box::{KeptBox, Part};
use crate::layout::BoxRect;
use crate::paint_order::Step;

/// The items of a paint, in painting order, each with its clip.
#[derive(Clone, Copy)]
pub struct SceneView<'a> {
    steps: &'a [Step],
    boxes: &'a [Option<KeptBox>],
}

impl<'a> SceneView<'a> {
    /// The view of `boxes`' parts walked through `steps`.
    pub(crate) fn new(steps: &'a [Step], boxes: &'a [Option<KeptBox>]) -> Self {
        Self { steps, boxes }
    }

    /// Each item, first drawn first, with its clip (device pixels).
    pub fn items(&self) -> impl Iterator<Item = (&'a Item, Option<Rect>)> + 'a {
        let boxes = self.boxes;
        self.steps.iter().flat_map(move |&step| {
            let scene = step_scene(boxes, step);
            let clips = scene.clips().iter().copied();
            scene.items().iter().zip(clips)
        })
    }

    /// The batches for a target whose drawable area is `view` (device
    /// pixels): [`vapors_scene::batched`] over [`SceneView::items`].
    #[must_use]
    pub fn batches(&self, view: Rect) -> Batched {
        vapors_scene::batched(self.items(), view)
    }

    /// The items as a flat scene, each under its clip.
    #[must_use]
    pub fn to_scene(&self) -> Scene {
        let mut scene = Scene::new();
        for (item, clip) in self.items() {
            scene.set_clip(clip);
            scene.push(*item);
        }
        scene
    }

    /// Each step with its items, for the debug build's check to name
    /// the step an item is from.
    pub(crate) fn steps(&self) -> impl Iterator<Item = (Step, &'a Scene)> + 'a {
        let boxes = self.boxes;
        self.steps
            .iter()
            .map(move |&step| (step, step_scene(boxes, step)))
    }

    /// Every hit, in painting order.
    pub(crate) fn hits(&self) -> impl Iterator<Item = &'a (NodeId, BoxRect)> + 'a {
        self.parts().flat_map(|part| part.hits.iter())
    }

    /// Each box step's part, in painting order.
    fn parts(&self) -> impl Iterator<Item = &'a Part> + 'a {
        let boxes = self.boxes;
        self.steps.iter().filter_map(move |&step| match step {
            Step::Box(slot) => Some(&kept(boxes, slot, step).own),
            Step::Thumb(_) | Step::Outline(_) => None,
        })
    }
}

/// The items `step` paints: its slot's own part, outline or thumbs.
fn step_scene(boxes: &[Option<KeptBox>], step: Step) -> &Scene {
    let kept = kept(boxes, step.index(), step);
    match step {
        Step::Box(_) => &kept.own.scene,
        Step::Thumb(_) => &kept.thumb,
        Step::Outline(_) => &kept.outline,
    }
}

/// Slot `slot`'s kept paint, or a panic naming the step and slot.
fn kept(boxes: &[Option<KeptBox>], slot: usize, step: Step) -> &KeptBox {
    match boxes.get(slot) {
        Some(Some(kept)) => kept,
        _ => panic!(
            "paint step {step:?} names slot {slot}, which holds no kept paint: \
             a painted box was not kept or a freed slot is still in paint order \
             (paint_partial.rs, kept_order.rs); reproduce with `cargo xtask test`"
        ),
    }
}
