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
pub(crate) use super::stacking::{Stacking, stacking_of};

/// One step of painting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Step {
    /// A box's own painting.
    Box(usize),
    /// A scroll container's thumb.
    Thumb(usize),
    /// A box's outline (3-E2b): last within its stacking context, as
    /// CSS 2 Appendix E paints outlines.
    Outline(usize),
}

/// The boxes, box 0 the root, in paint order, with each scroll
/// container's thumb.
#[cfg(test)]
pub(crate) fn paint_steps(boxes: &[LayoutBox], stacking: &dyn Fn(usize) -> Stacking) -> Vec<Step> {
    steps(boxes, stacking, false)
}

/// [`paint_steps`] with each stacking context's outlines after it: what
/// the painter walks.
pub(crate) fn paint_steps_with_outlines(
    boxes: &[LayoutBox],
    stacking: &dyn Fn(usize) -> Stacking,
) -> Vec<Step> {
    steps(boxes, stacking, true)
}

fn steps(boxes: &[LayoutBox], stacking: &dyn Fn(usize) -> Stacking, outlines: bool) -> Vec<Step> {
    let mut walk = Walk {
        boxes,
        stacking,
        out: Vec::with_capacity(boxes.len()),
        outlines: outlines.then(Vec::new),
    };
    walk.context(0);
    crate::thumb_order::thumbs_after_descendants(&walk.out, boxes)
}

struct Walk<'a> {
    boxes: &'a [LayoutBox],
    stacking: &'a dyn Fn(usize) -> Stacking,
    out: Vec<Step>,
    /// When outlines are wanted: the boxes painted so far in each open
    /// stacking context, innermost last.
    outlines: Option<Vec<Vec<usize>>>,
}

impl Walk<'_> {
    fn children(&self, index: usize) -> Vec<usize> {
        self.boxes[index].children.clone()
    }

    /// A stacking context rooted at `root`.
    fn context(&mut self, root: usize) {
        if let Some(frames) = &mut self.outlines {
            frames.push(Vec::new());
        }
        self.boxed(root);
        let layers = self.layers(root);
        let negative = layers.partition_point(|&layer| self.z(layer) < 0);
        for &layer in &layers[..negative] {
            self.layer(layer);
        }
        for child in self.children(root) {
            self.flow(child);
        }
        // outlines with their own layer, before the positioned layers
        // above it, as Blink paints them (review-v82)
        self.flush_outlines();
        for &layer in &layers[negative..] {
            self.layer(layer);
        }
        self.thumb(root);
    }

    /// The open frame's outlines, closing it.
    fn flush_outlines(&mut self) {
        if let Some(frame) = self.outlines.as_mut().and_then(Vec::pop) {
            self.out.extend(frame.into_iter().map(Step::Outline));
        }
    }

    /// `index`'s own painting, kept for its context's outlines.
    fn boxed(&mut self, index: usize) {
        self.out.push(Step::Box(index));
        if let Some(frame) = self.outlines.as_mut().and_then(|frames| frames.last_mut()) {
            frame.push(index);
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
        self.whole(index);
    }

    /// A positioned box: its own context, or painted whole in place.
    fn layer(&mut self, index: usize) {
        if (self.stacking)(index).roots_context() {
            return self.context(index);
        }
        // a positioned box with z-index auto is a layer of its own for
        // its outlines, though not a stacking context
        if let Some(frames) = &mut self.outlines {
            frames.push(Vec::new());
        }
        self.whole(index);
        self.flush_outlines();
    }

    /// `index`, its in-flow descendants, then its thumb if it scrolls.
    fn whole(&mut self, index: usize) {
        self.boxed(index);
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
