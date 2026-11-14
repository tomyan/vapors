//! What paint order needs of each box's style (`paint_order`): whether
//! it is positioned, its z-index, whether it is fixed and whether it
//! scrolls; and what decides containing blocks, which kept paint
//! compares to know when to walk them again (`changed_slots.rs`).

use super::layout::{Content, Key, LayoutBox, Rebuilt};
use crate::slot_list;
use crate::style::{Used, UsedMap};

/// What paint order needs to know of a box.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Stacking {
    /// `position` is not static.
    pub(crate) positioned: bool,
    /// `z-index`, `None` for auto.
    pub(crate) z: Option<i32>,
    /// `position: fixed`.
    pub(crate) fixed: bool,
    /// A scroll container, which paints a thumb.
    pub(crate) scrolls: bool,
    /// What decides containing blocks.
    pub(crate) placement: Placement,
}

/// What decides which box is a box's containing block: compared, not
/// read, by paint order (`changed_slots.rs`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Placement {
    /// The containing block of absolute boxes, as Taffy hoists them
    /// (`CoreStyle::is_containing_block`).
    contains_absolute: bool,
    /// The containing block of fixed boxes, likewise.
    // No test fails when this field is left out of the comparison:
    // Taffy 0.15's default `is_containing_block`, which `taffy::Style`
    // uses, never claims fixed boxes, so it is false on every box. It
    // becomes testable when a property that makes a box the containing
    // block of fixed boxes (`transform`, `filter`, `contain: paint`) is
    // implemented through a `CoreStyle` that claims them: a test then
    // sets that property on a rebuilt box with its children kept.
    contains_fixed: bool,
    /// `position` absolute or fixed: placed against its containing
    /// block, not its parent.
    out_of_flow: bool,
}

impl Placement {
    fn of(layout: &taffy::Style) -> Self {
        let claims = taffy::CoreStyle::is_containing_block(layout);
        Self {
            contains_absolute: claims.absolute,
            contains_fixed: claims.fixed,
            out_of_flow: layout.position.is_out_of_flow(),
        }
    }
}

impl Stacking {
    pub(crate) fn roots_context(self) -> bool {
        self.positioned && (self.z.is_some() || self.fixed)
    }
}

/// Each slot's stacking, so paint order and scroll chains read a slot
/// in place of looking its node's style up on every visit; `None` for
/// a vacant slot. Kept between updates and made again only for the
/// rebuilt slots (`patched`): a stacking comes from the used style,
/// and an element whose computed style changes is marked and its box
/// built again (`restyle_pass.rs`). The used `z-index` also depends on
/// the parent's used display (a flex item's z-index applies when static), which can
/// change while the element's computed style does not; that holds
/// because the parent is marked and a mark builds its whole subtree
/// again (`layout/marks.rs`). A used value that depends on something
/// outside the subtree (a sibling, the viewport) needs its own check.
#[derive(Default)]
pub(crate) struct Stackings(Vec<Option<Stacking>>);

impl Stackings {
    /// Its per-slot list's length and capacity, for the growth test.
    #[cfg(test)]
    pub(crate) fn slot_lists(&self) -> Vec<(&'static str, usize, usize)> {
        vec![("stackings", self.0.len(), self.0.capacity())]
    }

    /// The stackings of `boxes`, from their elements' used styles.
    pub(crate) fn of(boxes: &[LayoutBox], used: &UsedMap) -> Self {
        Self(
            boxes
                .iter()
                .map(|layout_box| stacking_of(layout_box, used))
                .collect(),
        )
    }

    /// `self`, the last update's stackings, with each `rebuilt` slot's
    /// made again from `boxes` and each slot freed and not taken again
    /// cleared; and what that found. A kept box's containing block
    /// changes only through a rebuilt box: itself, or an ancestor whose
    /// mark rebuilt the subtree.
    pub(crate) fn patched(
        mut self,
        boxes: &[LayoutBox],
        used: &UsedMap,
        rebuilt: &Rebuilt,
    ) -> (Self, Patched) {
        slot_list::resize(&mut self.0, boxes.len(), None);
        let restacked = self.make_again(boxes, used, &rebuilt.slots);
        let freed_stacked = self.clear_freed(boxes, &rebuilt.freed);
        (
            self,
            Patched {
                restacked,
                freed_stacked,
            },
        )
    }

    /// Makes each of `slots`' stackings again from `boxes`; whether one
    /// differs from what the slot held (a newly taken slot held none).
    fn make_again(&mut self, boxes: &[LayoutBox], used: &UsedMap, slots: &[usize]) -> bool {
        let mut restacked = false;
        for &slot in slots {
            let fresh = stacking_of(&boxes[slot], used);
            restacked |= self.0[slot].is_some_and(|held| Some(held) != fresh);
            self.0[slot] = fresh;
        }
        restacked
    }

    /// Clears each `freed` slot that no box took again; whether one
    /// held a stacking other than the default.
    fn clear_freed(&mut self, boxes: &[LayoutBox], freed: &[(usize, Key)]) -> bool {
        let mut stacked = false;
        for &(slot, _) in freed {
            if matches!(boxes[slot].content, Content::Vacant) {
                stacked |= self.0[slot].take() != Some(Stacking::default());
            }
        }
        stacked
    }

    /// How many slots hold a stacking.
    pub(crate) fn held(&self) -> usize {
        self.0.iter().flatten().count()
    }

    /// Panics, naming the first differing slot, unless `self` holds the
    /// stackings of `fresh`: the debug check of `patched`.
    pub(crate) fn assert_matches(&self, fresh: &Self) {
        let slots = self.0.len().max(fresh.0.len());
        let Some(slot) = (0..slots).find(|&slot| self.at(slot) != fresh.at(slot)) else {
            return;
        };
        panic!(
            "slot {slot}'s kept stacking is {:?}, a fresh one {:?}: a box whose stacking \
             changed was not built again (`restyle_pass.rs` marks it); reproduce: \
             cargo nextest run -p vapors-elements",
            self.at(slot),
            fresh.at(slot),
        );
    }

    /// Box `index`'s stacking. Panics on a vacant slot, which is no
    /// box's child.
    pub(crate) fn get(&self, index: usize) -> Stacking {
        self.0[index].unwrap_or_else(|| unreachable!("slot {index} is vacant, and no box's child"))
    }

    /// Slot `index`'s stacking; `None` when vacant or past the end.
    pub(crate) fn at(&self, index: usize) -> Option<Stacking> {
        self.0.get(index).copied().flatten()
    }

    /// Whether box `index` is a scroll container.
    pub(crate) fn scrolls(&self, index: usize) -> bool {
        self.get(index).scrolls
    }
}

/// What [`Stackings::patched`] found, read by the order splice
/// (`order_splice.rs`): either true declines it.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Patched {
    /// A rebuilt slot's stacking differs from the one it held.
    pub(crate) restacked: bool,
    /// A freed slot held a stacking other than the default, so its
    /// steps were not in its parent's flow run.
    pub(crate) freed_stacked: bool,
}

/// A box's stacking, from its element's used style; `None` when vacant.
fn stacking_of(layout_box: &LayoutBox, used: &UsedMap) -> Option<Stacking> {
    let node = match layout_box.content {
        Content::Element(node) => node,
        Content::Text(_) => return Some(Stacking::default()),
        Content::Vacant => return None,
    };
    Some(stacking_of_style(&used[&node]))
}

/// An element's stacking, from its used style.
fn stacking_of_style(used: &Used) -> Stacking {
    Stacking {
        // a z-index is kept only where it applies: on a static box, a
        // flex or grid item, which then stacks as if positioned
        positioned: used.layout.position != taffy::Position::Static || used.z_index.is_some(),
        z: used.z_index,
        fixed: used.layout.position == taffy::Position::Fixed,
        scrolls: used.scrolls(),
        placement: Placement::of(&used.layout),
    }
}

#[cfg(test)]
#[path = "stacking_tests.rs"]
mod tests;
