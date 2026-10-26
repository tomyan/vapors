//! What paint order needs of each box's style (`paint_order`): whether
//! it is positioned, its z-index, whether it is fixed and whether it
//! scrolls.

use super::layout::LayoutBox;

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

/// Box `index`'s stacking, from its element's used style.
pub(crate) fn stacking_of(
    boxes: &[LayoutBox],
    index: usize,
    used: &std::collections::HashMap<vapors_dom::NodeId, crate::style::Used>,
) -> Stacking {
    let crate::layout::Content::Element(node) = boxes[index].content else {
        return Stacking::default();
    };
    let used = &used[&node];
    Stacking {
        // a z-index is kept only where it applies: on a static box, a
        // flex or grid item, which then stacks as if positioned
        positioned: used.layout.position != taffy::Position::Static || used.z_index.is_some(),
        z: used.z_index,
        fixed: used.layout.position == taffy::Position::Fixed,
        scrolls: used.scrolls(),
    }
}
