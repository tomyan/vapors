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
}

impl Stacking {
    fn roots_context(self) -> bool {
        self.positioned && (self.z.is_some() || self.fixed)
    }
}

/// The boxes, box 0 the root, in paint order.
pub(crate) fn paint_order(boxes: &[LayoutBox], stacking: &dyn Fn(usize) -> Stacking) -> Vec<usize> {
    let mut walk = Walk {
        boxes,
        stacking,
        out: Vec::with_capacity(boxes.len()),
    };
    walk.context(0);
    walk.out
}

struct Walk<'a> {
    boxes: &'a [LayoutBox],
    stacking: &'a dyn Fn(usize) -> Stacking,
    out: Vec<usize>,
}

impl Walk<'_> {
    fn children(&self, index: usize) -> Vec<usize> {
        self.boxes[index].children.clone()
    }

    /// A stacking context rooted at `root`.
    fn context(&mut self, root: usize) {
        self.out.push(root);
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
        self.out.push(index);
        for child in self.children(index) {
            self.flow(child);
        }
    }

    /// A positioned box: its own context, or painted whole in place.
    fn layer(&mut self, index: usize) {
        if (self.stacking)(index).roots_context() {
            return self.context(index);
        }
        self.out.push(index);
        for child in self.children(index) {
            self.flow(child);
        }
    }
}

#[cfg(test)]
#[path = "paint_order_tests.rs"]
mod tests;
