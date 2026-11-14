//! Painting the slots an update paints again into parts kept in place
//! (`paint_partial.rs`), and the thumbs of the scroll containers it
//! names (`thumbs_again.rs`).

use std::mem::take;

use vapors_scene::Scene;

use crate::kept_box::{BoxFrom, KeptBox, Part};
use crate::kept_order::KeptOrder;
use crate::layout::{BoxRect, LayoutBox};
use crate::paint::{PaintInput, clip_to, paint_box};
use crate::painted::Painted;
use crate::painting::Painting;

/// Paints boxes one at a time, each under its kept clip.
pub(crate) struct Repaint<'a> {
    boxes: &'a [LayoutBox],
    rects: &'a [BoxRect],
    order: &'a KeptOrder,
    /// Where a box is painted, before it is kept.
    scratch: Painting,
}

impl<'a> Repaint<'a> {
    pub(crate) fn new(boxes: &'a [LayoutBox], rects: &'a [BoxRect], order: &'a KeptOrder) -> Self {
        Self {
            boxes,
            rects,
            order,
            scratch: Painting::default(),
        }
    }

    /// Paints each slot of `again`, made from what it reads, and keeps
    /// its part in `painted`, its entries put into the maps; counted.
    pub(crate) fn boxes(
        &mut self,
        again: Vec<(usize, BoxFrom)>,
        input: &mut PaintInput<'_>,
        painted: &mut Painted,
    ) {
        for (slot, from) in again {
            let kept = self.painted(slot, from, input, painted);
            kept.own.enter(painted, kept.from.frame);
            painted.boxes_drawn += 1;
            painted.items_built += kept.items();
            painted.kept_boxes[slot] = Some(kept);
        }
    }

    /// Box `slot`'s own step and outline painted, their ids taken from
    /// `painted`'s slabs.
    fn painted(
        &mut self,
        slot: usize,
        from: BoxFrom,
        input: &mut PaintInput<'_>,
        painted: &mut Painted,
    ) -> KeptBox {
        self.scratch.slabs = take(&mut painted.slabs);
        let clip = self.order.clips.own(slot);
        clip_to(clip, input, &mut self.scratch.scene);
        paint_box(self.boxes, self.rects, slot, input, &mut self.scratch);
        let own = Part::take(&mut self.scratch);
        clip_to(clip, input, &mut self.scratch.scene);
        let (layout_box, rect) = (&self.boxes[slot], self.rects[slot]);
        crate::paint_outline::paint_outline(layout_box, rect, input, &mut self.scratch);
        painted.slabs = take(&mut self.scratch.slabs);
        KeptBox {
            from,
            own,
            outline: take(&mut self.scratch.scene),
            thumb: Scene::new(),
        }
    }

    /// Paints the thumbs of the scroll containers `slots` into their
    /// kept paint; their items counted.
    pub(crate) fn thumbs(
        &mut self,
        slots: &[usize],
        input: &mut PaintInput<'_>,
        painted: &mut Painted,
    ) {
        for &slot in slots {
            clip_to(self.order.clips.own(slot), input, &mut self.scratch.scene);
            let (layout_box, rect) = (&self.boxes[slot], self.rects[slot]);
            crate::thumb::paint(layout_box, rect, input, &mut self.scratch);
            let thumb = take(&mut self.scratch.scene);
            painted.items_built += thumb.items().len();
            kept_mut(painted, slot).thumb = thumb;
        }
    }
}

/// Slot `slot`'s kept paint, or a panic naming the slot.
fn kept_mut(painted: &mut Painted, slot: usize) -> &mut KeptBox {
    painted.kept_boxes[slot].as_mut().unwrap_or_else(|| {
        panic!(
            "scroll container slot {slot} has thumbs but no kept paint: its box step \
             was not painted (paint_partial.rs, kept_order.rs); reproduce with `cargo xtask test`"
        )
    })
}
