//! The debug build's check on a kept paint's scene and slabs, part of
//! `kept_paint_check.rs`: item by item against a fresh paint's, each
//! id resolved, since a kept part's ids are not the ones a fresh paint
//! gives; and each held slab id named by exactly one kept part.

use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use vapors_scene::{IconId, ImageId, Item, Rect, Scene, SurfaceId, TextId};

use crate::paint_order::Step;
use crate::painted::Painted;
use crate::slab::{Named, Slabs};

/// One scene item with its clip and the step that painted it.
type Placed = (Item, Option<Rect>, Step);

/// The first scene item at which `kept` and `fresh` differ, ids
/// resolved, with the steps (each naming its slot) that painted it.
pub(crate) fn first_differing_item(kept: &Painted, fresh: &Painted) -> Option<String> {
    let (a, b) = (placed(kept), placed(fresh));
    let at = (0..a.len().max(b.len())).find(|&at| !same_at(kept, fresh, (a.get(at), b.get(at))))?;
    Some(format!(
        "scene item {at}: kept {:?}, fresh {:?}",
        a.get(at),
        b.get(at)
    ))
}

/// Each item of `painted`'s scene with its clip and step.
fn placed(painted: &Painted) -> Vec<Placed> {
    painted.scene().steps().flat_map(step_items).collect()
}

/// `scene`'s items, each with its clip and `step`.
fn step_items((step, scene): (Step, &Scene)) -> impl Iterator<Item = Placed> + '_ {
    let clips = scene.clips().iter();
    let items = scene.items().iter().zip(clips);
    items.map(move |(item, clip)| (*item, *clip, step))
}

/// Whether two placed items and their clips are the same, ids resolved.
fn same_at(kept: &Painted, fresh: &Painted, items: (Option<&Placed>, Option<&Placed>)) -> bool {
    match items {
        (Some((a, clip_a, _)), Some((b, clip_b, _))) => {
            clip_a == clip_b && same_item((kept, a), (fresh, b))
        }
        _ => false,
    }
}

/// Whether two items are the same but for their ids, and those ids
/// name the same layout, image or icon (`Rc::ptr_eq`) or surface name.
fn same_item((kept, a): (&Painted, &Item), (fresh, b): (&Painted, &Item)) -> bool {
    match (Named::of(a), Named::of(b)) {
        (None, None) => a == b,
        (Some(x), Some(y)) => unnamed(*a) == unnamed(*b) && same_target(kept, x, fresh, y),
        _ => false,
    }
}

fn same_target(kept: &Painted, x: Named, fresh: &Painted, y: Named) -> bool {
    let (k, f) = (&kept.slabs, &fresh.slabs);
    match (x, y) {
        (Named::Text(x), Named::Text(y)) => same_rc(k.texts.get(x), f.texts.get(y)),
        (Named::Image(x), Named::Image(y)) => same_rc(k.images.get(x), f.images.get(y)),
        (Named::Icon(x), Named::Icon(y)) => same_rc(k.icons.get(x), f.icons.get(y)),
        (Named::Surface(x), Named::Surface(y)) => {
            k.surfaces.get(x).is_some() && k.surfaces.get(x) == f.surfaces.get(y)
        }
        _ => false,
    }
}

fn same_rc<T>(a: Option<&Rc<T>>, b: Option<&Rc<T>>) -> bool {
    matches!((a, b), (Some(a), Some(b)) if Rc::ptr_eq(a, b))
}

/// `item` with its id, if it names one, set to 0.
fn unnamed(item: Item) -> Item {
    match item {
        Item::Text { origin, ink, .. } => Item::Text {
            text: TextId(0),
            origin,
            ink,
        },
        Item::Image { rect, .. } => Item::Image {
            image: ImageId(0),
            rect,
        },
        Item::Icon { rect, colour, .. } => Item::Icon {
            icon: IconId(0),
            rect,
            colour,
        },
        Item::Surface { rect, .. } => Item::Surface {
            surface: SurfaceId(0),
            rect,
        },
        Item::Shadow(_) | Item::Quad(_) | Item::Shape(_) => item,
    }
}

/// The slots naming each slab id among the kept parts' items.
fn namers<'a>(parts: impl Iterator<Item = (usize, &'a Item)>) -> BTreeMap<Named, Vec<usize>> {
    let mut namers: BTreeMap<Named, Vec<usize>> = BTreeMap::new();
    for (slot, item) in parts {
        if let Some(named) = Named::of(item) {
            namers.entry(named).or_default().push(slot);
        }
    }
    namers
}

/// The first way `slabs` and the items the kept `parts` hold (each with
/// its slot) disagree: an id a part names that is free, or a held id
/// not named by exactly one part.
pub(crate) fn slab_difference<'a>(
    slabs: &Slabs,
    parts: impl Iterator<Item = (usize, &'a Item)>,
) -> Option<String> {
    let namers = namers(parts);
    let live: BTreeSet<Named> = slabs.live().collect();
    if let Some((named, slots)) = namers.iter().find(|(named, _)| !live.contains(named)) {
        return Some(format!(
            "slab id {named:?} is free, yet named by slots {slots:?}"
        ));
    }
    live.iter().find_map(|named| {
        let slots = namers.get(named).map_or(&[][..], Vec::as_slice);
        (slots.len() != 1)
            .then(|| format!("slab id {named:?} is named by slots {slots:?}, not one"))
    })
}

/// The first slot that holds a kept part but has no box step: a part
/// that is never shown, whose ids stay held.
pub(crate) fn part_off_steps(painted: &Painted) -> Option<String> {
    let mut stepped = vec![false; painted.kept_boxes.len()];
    let boxes = painted
        .order
        .steps
        .iter()
        .filter(|step| matches!(step, Step::Box(_)));
    for step in boxes {
        if let Some(seen) = stepped.get_mut(step.index()) {
            *seen = true;
        }
    }
    let parts = painted.kept_boxes.iter().map(Option::is_some);
    let slot = parts
        .zip(stepped)
        .position(|(part, stepped)| part && !stepped)?;
    Some(format!("slot {slot} holds a kept part but has no box step"))
}

#[cfg(test)]
#[path = "kept_scene_check_tests.rs"]
mod tests;
