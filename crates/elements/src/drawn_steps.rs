//! The paint steps that draw something, in paint order (review v201):
//! batching walks only these, so it reads neither the steps that paint
//! nothing nor their slots' kept paint. Made afresh when paint order is
//! walked again; else patched for the slots painted again or freed. A
//! debug build compares it with one made afresh (`kept_paint_check.rs`).

use crate::kept_box::KeptBox;
use crate::paint_order::Step;

/// Each slot's steps' places in paint order, and the steps whose items
/// are not empty, each with its place.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct DrawnSteps {
    /// By slot: the places of its box, thumb and outline steps.
    places: Vec<[Option<usize>; 3]>,
    /// The steps that draw, ordered by place.
    drawn: Vec<(usize, Step)>,
}

impl DrawnSteps {
    /// The steps of `steps` whose part in `boxes` holds items.
    pub(crate) fn of(steps: &[Step], boxes: &[Option<KeptBox>]) -> Self {
        let mut places = vec![[None; 3]; boxes.len()];
        for (place, &step) in steps.iter().enumerate() {
            if let Some(slot) = places.get_mut(step.index()) {
                slot[kind(step)] = Some(place);
            }
        }
        let places_drawn = steps.iter().copied().enumerate();
        let drawn = places_drawn
            .filter(|&(_, step)| draws(boxes, step))
            .collect();
        Self { places, drawn }
    }

    /// `self` with the steps of each of `slots` put in or taken out as
    /// its part in `boxes` now holds items or not.
    pub(crate) fn patch(&mut self, slots: impl Iterator<Item = usize>, boxes: &[Option<KeptBox>]) {
        for slot in slots {
            let placed: Vec<(usize, Step)> = self.places_of(slot).collect();
            for (place, step) in placed {
                self.patch_step(place, step, boxes);
            }
        }
    }

    /// Puts in or takes out `step`, at `place`, as it draws or not.
    fn patch_step(&mut self, place: usize, step: Step, boxes: &[Option<KeptBox>]) {
        let found = self.drawn.binary_search_by_key(&place, |&(place, _)| place);
        match (found, draws(boxes, step)) {
            (Err(at), true) => self.drawn.insert(at, (place, step)),
            (Ok(at), false) => _ = self.drawn.remove(at),
            _ => {}
        }
    }

    /// The steps that draw, in paint order.
    pub(crate) fn steps(&self) -> impl Iterator<Item = Step> + '_ {
        self.drawn.iter().map(|&(_, step)| step)
    }

    /// The steps that draw, each with its place, in paint order.
    pub(crate) fn placed(&self) -> impl Iterator<Item = (usize, Step)> + '_ {
        self.drawn.iter().copied()
    }

    /// The place of `step` in paint order, if it is in it.
    pub(crate) fn place_of(&self, step: Step) -> Option<usize> {
        self.places.get(step.index())?[kind(step)]
    }

    /// Slot `slot`'s steps that are in paint order, each with its place.
    pub(crate) fn places_of(&self, slot: usize) -> impl Iterator<Item = (usize, Step)> {
        let places = self.places.get(slot).copied().unwrap_or_default();
        let steps = places.into_iter().zip(steps_of(slot));
        steps.filter_map(|(place, step)| Some((place?, step)))
    }
}

/// True when `step`'s slot holds a kept paint whose items for `step`
/// are not empty.
pub(crate) fn draws(boxes: &[Option<KeptBox>], step: Step) -> bool {
    let kept = boxes.get(step.index()).and_then(Option::as_ref);
    kept.is_some_and(|kept| !kept.scene_of(step).items().is_empty())
}

/// The index of `step`'s kind in a slot's places.
fn kind(step: Step) -> usize {
    match step {
        Step::Box(_) => 0,
        Step::Thumb(_) => 1,
        Step::Outline(_) => 2,
    }
}

/// Slot `slot`'s steps, in the order of [`kind`].
fn steps_of(slot: usize) -> [Step; 3] {
    [Step::Box(slot), Step::Thumb(slot), Step::Outline(slot)]
}

#[cfg(test)]
#[path = "drawn_steps_tests.rs"]
mod tests;
