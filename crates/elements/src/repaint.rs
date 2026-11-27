//! Painting the slots an update paints again into parts kept in place
//! (`paint_partial.rs`), and the thumbs of the scroll containers it
//! names (`thumbs_again.rs`).

use std::mem::take;

use crate::kept_box::{BoxFrom, KeptBox, Part};
use crate::kept_order::KeptOrder;
use crate::layout::{BoxRect, LayoutBox};
use crate::paint::{PaintInput, clip_to, paint_box};
use crate::paint_refusals::{NotPlaced, PartKind};
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
            if let Some(why) = self.in_rounded_frame(slot, from, input, painted) {
                input.frame_refused.push((slot, why));
            }
            let kept = self.painted(slot, from, input, painted);
            self.check_placed(slot, &kept, input);
            input.class = None;
            kept.own.enter(painted, kept.from.frame);
            painted.boxes_drawn += 1;
            painted.items_built += kept.items();
            painted.kept_boxes[slot] = Some(kept);
        }
    }

    /// What box `slot`'s rounded frame refuses of it, if it is in one
    /// (`clip_refusals.rs` `in_frame`): its region the clip it gives its
    /// content, else its border box, on the page.
    fn in_rounded_frame(
        &self,
        slot: usize,
        from: BoxFrom,
        input: &PaintInput<'_>,
        painted: &Painted,
    ) -> Option<crate::clip_refusals::RoundedRefusal> {
        let round = painted.frames.at(from.frame).rounded?;
        let framed = matches!(input.placement.framing.get(&slot), Some(Ok(())));
        let a_frame = input.stackings.scrolls(slot) || framed;
        let region = self
            .order
            .clips
            .inner(slot)
            .map_or(from.rect, |clip| clip.rect);
        let region = painted.frames.on_page(from.frame, region);
        crate::clip_refusals::in_frame(&self.boxes[slot], round, (a_frame, region))
    }

    /// Records box `slot`'s part as not placed when a coordinate is not
    /// finite; a debug build checks a placed one is painted whole.
    fn check_placed(&self, slot: usize, kept: &KeptBox, input: &mut PaintInput<'_>) {
        let border_box = input.snapped(self.rects[slot]);
        match not_placed(kept, border_box) {
            Some(part) => input.unresolvable.push(NotPlaced { slot, part }),
            #[cfg(debug_assertions)]
            None => crate::repaint_check::assert_painted_whole(kept, border_box, slot, input),
            #[cfg(not(debug_assertions))]
            None => {}
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
        clip_to(
            clip,
            (self.boxes, self.rects),
            input,
            &mut self.scratch.scene,
        );
        paint_box(self.boxes, self.rects, slot, input, &mut self.scratch);
        let own = Part::take(&mut self.scratch);
        clip_to(
            clip,
            (self.boxes, self.rects),
            input,
            &mut self.scratch.scene,
        );
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
            let clip = self.order.clips.own(slot);
            clip_to(
                clip,
                (self.boxes, self.rects),
                input,
                &mut self.scratch.scene,
            );
            let (layout_box, rect) = (&self.boxes[slot], self.rects[slot]);
            crate::thumb::paint(layout_box, rect, input, &mut self.scratch);
            let thumb = take(&mut self.scratch.scene);
            if thumb.non_finite().is_some() {
                let part = PartKind::Thumb;
                input.unresolvable.push(NotPlaced { slot, part });
            } else {
                #[cfg(debug_assertions)]
                crate::repaint_check::assert_snapped_whole(thumb.items().iter(), slot, input);
            }
            input.class = None;
            painted.items_built += thumb.items().len();
            kept_mut(painted, slot).set_thumb(thumb);
        }
    }
}

/// What of a box's paint holds a coordinate that is not finite: its
/// border box, its own step's items or clips, or its outline ring;
/// `None` when every one is finite.
fn not_placed(kept: &KeptBox, border_box: vapors_scene::Rect) -> Option<PartKind> {
    if !crate::paint::finite(border_box) {
        return Some(PartKind::BorderBox);
    }
    if let Some(found) = kept.own.scene.non_finite() {
        return Some(PartKind::Own(found.kind));
    }
    kept.outline().non_finite().map(|_| PartKind::Outline)
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
