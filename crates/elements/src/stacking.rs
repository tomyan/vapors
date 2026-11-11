//! What paint order needs of each box's style (`paint_order`): whether
//! it is positioned, its z-index, whether it is fixed and whether it
//! scrolls.

use super::layout::{Content, LayoutBox};
use crate::node_map::NodeMap;
use crate::style::Used;

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
}

impl Stacking {
    pub(crate) fn roots_context(self) -> bool {
        self.positioned && (self.z.is_some() || self.fixed)
    }
}

/// Each slot's stacking, made once per update so paint order and
/// scroll chains read a slot in place of looking its node's style up on
/// every visit; `None` for a vacant slot.
#[derive(Default)]
pub(crate) struct Stackings(Vec<Option<Stacking>>);

impl Stackings {
    /// The stackings of `boxes`, from their elements' used styles.
    pub(crate) fn of(boxes: &[LayoutBox], used: &NodeMap<Used>) -> Self {
        Self(
            boxes
                .iter()
                .map(|layout_box| stacking_of(layout_box, used))
                .collect(),
        )
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

/// A box's stacking, from its element's used style; `None` when vacant.
fn stacking_of(layout_box: &LayoutBox, used: &NodeMap<Used>) -> Option<Stacking> {
    let node = match layout_box.content {
        Content::Element(node) => node,
        Content::Text(_) => return Some(Stacking::default()),
        Content::Vacant => return None,
    };
    let used = &used[&node];
    Some(Stacking {
        // a z-index is kept only where it applies: on a static box, a
        // flex or grid item, which then stacks as if positioned
        positioned: used.layout.position != taffy::Position::Static || used.z_index.is_some(),
        z: used.z_index,
        fixed: used.layout.position == taffy::Position::Fixed,
        scrolls: used.scrolls(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "slot 0 is vacant, and no box's child")]
    fn a_vacant_slots_stacking_is_refused() {
        // Given: one vacant slot
        let stackings = Stackings::of(&[LayoutBox::vacant()], &NodeMap::default());

        // When: its stacking is asked for
        let _ = stackings.get(0);

        // Then: it panics, naming the slot
    }
}
