//! Painting the slots an update paints again into parts kept in place
//! (`paint_partial.rs`), and the thumbs of the scroll containers it
//! names (`thumbs_again.rs`).

use std::mem::take;

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
            let border_box = input.snapped(self.rects[slot]);
            if crate::paint::finite(border_box) {
                #[cfg(debug_assertions)]
                crate::repaint_check::assert_painted_whole(&kept, border_box, slot, input);
            } else {
                input.unresolvable.push((slot, self.rects[slot]));
            }
            input.class = None;
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
        input.class = Some(painted.frames.class_of(from.frame));
        let clip = self.order.clips.own(slot);
        clip_to(clip, input, &mut self.scratch.scene);
        paint_box(self.boxes, self.rects, slot, input, &mut self.scratch);
        let own = Part::take(&mut self.scratch);
        clip_to(clip, input, &mut self.scratch.scene);
        let (layout_box, rect) = (&self.boxes[slot], self.rects[slot]);
        crate::paint_outline::paint_outline(layout_box, rect, input, &mut self.scratch);
        painted.slabs = take(&mut self.scratch.slabs);
        KeptBox::new(from, own, take(&mut self.scratch.scene))
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
            let frame = kept_mut(painted, slot).from.frame;
            input.class = Some(painted.frames.class_of(frame));
            clip_to(self.order.clips.own(slot), input, &mut self.scratch.scene);
            let (layout_box, rect) = (&self.boxes[slot], self.rects[slot]);
            crate::thumb::paint(layout_box, rect, input, &mut self.scratch);
            let thumb = take(&mut self.scratch.scene);
            // a box paint refuses (`unresolvable`) has thumbs from its
            // infinite rect; the update is refused, so they are not checked
            #[cfg(debug_assertions)]
            if !input
                .unresolvable
                .iter()
                .any(|&(refused, _)| refused == slot)
            {
                crate::repaint_check::assert_snapped_whole(thumb.items().iter(), slot, input);
            }
            input.class = None;
            painted.items_built += thumb.items().len();
            kept_mut(painted, slot).set_thumb(thumb);
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
