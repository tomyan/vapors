//! The order boxes are painted in (F2c S8), CSS 2 Appendix E reduced to
//! what vapors lays out. Each stacking context paints its root's own
//! box; then its child contexts with a negative z-index, lowest first;
//! then its in-flow descendants in tree order, skipping positioned
//! ones; then its positioned descendants with a z-index of auto or 0,
//! in tree order; then its child contexts with a positive z-index. A
//! positioned box with z-index auto is painted whole at its place, but
//! the positioned boxes inside it belong to the enclosing context. A
//! positioned box with a z-index, and every fixed box, roots its own
//! context. Hit testing reads the same order backwards.
//!
//! A scroll container's thumb (F2c S9) paints after its in-flow
//! content, or, when it roots a stacking context, after the whole
//! context; then `thumb_order` moves it after any of its positioned
//! descendants that an enclosing context paints later.
//!
//! Within the in-flow descendants each box paints its own background
//! and then its children, where CSS paints every background before any
//! text: the two differ only where a later box's background overlaps
//! an earlier box's text.

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

/// One step of painting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Step {
    /// A box's own painting.
    Box(usize),
    /// A scroll container's thumb.
    Thumb(usize),
}

impl Stacking {
    fn roots_context(self) -> bool {
        self.positioned && (self.z.is_some() || self.fixed)
    }
}

/// The boxes, box 0 the root, in paint order, with each scroll
/// container's thumb.
pub(crate) fn paint_steps(boxes: &[LayoutBox], stacking: &dyn Fn(usize) -> Stacking) -> Vec<Step> {
    let mut walk = Walk {
        boxes,
        stacking,
        out: Vec::with_capacity(boxes.len()),
    };
    walk.context(0);
    crate::thumb_order::thumbs_after_descendants(&walk.out, boxes)
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

struct Walk<'a> {
    boxes: &'a [LayoutBox],
    stacking: &'a dyn Fn(usize) -> Stacking,
    out: Vec<Step>,
}

impl Walk<'_> {
    fn children(&self, index: usize) -> Vec<usize> {
        self.boxes[index].children.clone()
    }

    /// A stacking context rooted at `root`.
    fn context(&mut self, root: usize) {
        self.out.push(Step::Box(root));
        let layers = self.layers(root);
        let negative = layers.partition_point(|&layer| self.z(layer) < 0);
        for &layer in &layers[..negative] {
            self.layer(layer);
        }
        for child in self.children(root) {
            self.flow(child);
        }
        for &layer in &layers[negative..] {
            self.layer(layer);
        }
        self.thumb(root);
    }

    /// The positioned boxes of the context rooted at `root`, by z-index;
    /// the sort is stable, so equal z-indices, and auto with 0, stay in
    /// tree order.
    fn layers(&self, root: usize) -> Vec<usize> {
        let mut layers = Vec::new();
        for child in self.children(root) {
            self.collect(child, &mut layers);
        }
        layers.sort_by_key(|&layer| self.z(layer));
        layers
    }

    fn z(&self, index: usize) -> i32 {
        (self.stacking)(index).z.unwrap_or(0)
    }

    /// The positioned boxes at and below `index` that belong to the
    /// enclosing context, in tree order.
    fn collect(&self, index: usize, layers: &mut Vec<usize>) {
        let stacking = (self.stacking)(index);
        if stacking.positioned {
            layers.push(index);
            if stacking.roots_context() {
                return;
            }
        }
        for child in self.children(index) {
            self.collect(child, layers);
        }
    }

    /// `index` and its in-flow descendants, skipping positioned ones.
    fn flow(&mut self, index: usize) {
        if (self.stacking)(index).positioned {
            return;
        }
        self.whole(index);
    }

    /// A positioned box: its own context, or painted whole in place.
    fn layer(&mut self, index: usize) {
        if (self.stacking)(index).roots_context() {
            return self.context(index);
        }
        self.whole(index);
    }

    /// `index`, its in-flow descendants, then its thumb if it scrolls.
    fn whole(&mut self, index: usize) {
        self.out.push(Step::Box(index));
        for child in self.children(index) {
            self.flow(child);
        }
        self.thumb(index);
    }

    fn thumb(&mut self, index: usize) {
        if (self.stacking)(index).scrolls {
            self.out.push(Step::Thumb(index));
        }
    }
}

#[cfg(test)]
#[path = "paint_order_tests.rs"]
mod tests;
