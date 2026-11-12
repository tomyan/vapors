//! A paint's scene as a view over its kept paint order and each slot's
//! kept part (IU3 3c-3d2, `docs/plans/iu3.md` "The scene in stable
//! slots"): the items are each drawing step's part's, walked in order
//! (`drawn_steps.rs`), so no update copies a kept part into a flat list. Hit testing keeps the
//! parts' hits in the same order (`kept_hits.rs`). Each item is put on
//! the page from its part's scroll frame as it is read (`frames.rs`).

use vapors_scene::{Batched, Item, Rect, Scene};

use crate::drawn_steps::DrawnSteps;
use crate::frames::Frames;
use crate::kept_box::KeptBox;
use crate::kept_hits::FramedHit;
use crate::paint_order::Step;

/// The items of a paint, in painting order, each with its clip.
#[derive(Clone, Copy)]
pub struct SceneView<'a> {
    steps: &'a [Step],
    drawn: &'a DrawnSteps,
    boxes: &'a [Option<KeptBox>],
    frames: &'a Frames,
}

impl<'a> SceneView<'a> {
    /// The view of `boxes`' parts, in the scroll `frames`, walked
    /// through `steps`, whose steps that draw are `drawn`.
    pub(crate) fn new(
        steps: &'a [Step],
        drawn: &'a DrawnSteps,
        (boxes, frames): (&'a [Option<KeptBox>], &'a Frames),
    ) -> Self {
        Self {
            steps,
            drawn,
            boxes,
            frames,
        }
    }

    /// Each item on the page, first drawn first, with its clip (device
    /// pixels).
    pub fn items(&self) -> impl Iterator<Item = (Item, Option<Rect>)> + 'a {
        let (boxes, frames) = (self.boxes, self.frames);
        self.drawn
            .steps()
            .flat_map(move |step| step_items((boxes, frames), step))
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
            scene.push(item);
        }
        scene
    }

    /// Each step with its items in its frame, for the debug build's
    /// check to name the step an item is from.
    pub(crate) fn steps(&self) -> impl Iterator<Item = (Step, &'a Scene)> + 'a {
        let boxes = self.boxes;
        self.steps
            .iter()
            .map(move |&step| (step, step_scene(boxes, step)))
    }

    /// Every hit with its frame, in painting order.
    pub(crate) fn hits(&self) -> impl Iterator<Item = FramedHit> + 'a {
        let framed = |kept: &'a KeptBox| kept.own.hits.iter().map(|&hit| (kept.from.frame, hit));
        self.placed_parts().flat_map(move |(_, kept)| framed(kept))
    }

    /// Each box step's kept paint with the step's place, in painting
    /// order.
    pub(crate) fn placed_parts(&self) -> impl Iterator<Item = (usize, &'a KeptBox)> + 'a {
        let boxes = self.boxes;
        let placed = self.steps.iter().enumerate();
        placed.filter_map(move |(place, &step)| match step {
            Step::Box(slot) => Some((place, kept(boxes, slot, step))),
            Step::Thumb(_) | Step::Outline(_) => None,
        })
    }
}

/// The items `step` paints, each with its clip (device pixels), put on
/// the page from its part's frame in `frames`.
pub(crate) fn step_items<'a>(
    (boxes, frames): (&'a [Option<KeptBox>], &'a Frames),
    step: Step,
) -> impl Iterator<Item = (Item, Option<Rect>)> + 'a {
    let kept = kept(boxes, step.index(), step);
    let scene = kept.scene_of(step);
    let clips = scene.clips().iter().copied();
    let on_page = move |(item, clip)| frames.item(kept.from.frame, item, clip);
    scene.items().iter().zip(clips).map(on_page)
}

/// The items `step` paints: its slot's own part, outline or thumbs.
fn step_scene(boxes: &[Option<KeptBox>], step: Step) -> &Scene {
    kept(boxes, step.index(), step).scene_of(step)
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
