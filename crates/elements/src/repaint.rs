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
            #[cfg(debug_assertions)]
            assert_snapped_whole(kept.all_items(), slot, input);
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
        input.class = Some(painted.frames.at(from.frame).class);
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
            input.class = Some(painted.frames.at(frame).class);
            clip_to(self.order.clips.own(slot), input, &mut self.scratch.scene);
            let (layout_box, rect) = (&self.boxes[slot], self.rects[slot]);
            crate::thumb::paint(layout_box, rect, input, &mut self.scratch);
            let thumb = take(&mut self.scratch.scene);
            #[cfg(debug_assertions)]
            assert_snapped_whole(thumb.items().iter(), slot, input);
            painted.items_built += thumb.items().len();
            kept_mut(painted, slot).set_thumb(thumb);
        }
    }
}

/// Panics, naming the slot and the item, when an item of the box in
/// `slot` that is snapped (`snapped_kind`) is not whole device pixels
/// in its frame while the frame is a whole translation, so that it
/// would land between pixels on the page. Text is placed unsnapped and
/// its glyphs quantised by the painter; a shape's geometry is in its
/// transform, unsnapped as Chromium paints SVG.
#[cfg(debug_assertions)]
fn assert_snapped_whole<'i>(
    items: impl Iterator<Item = &'i vapors_scene::Item>,
    slot: usize,
    input: &PaintInput<'_>,
) {
    use crate::kept_shift_items::{snapped_kind, whole_where_finite};
    if !input.class.is_some_and(vapors_scene::TransformClass::snaps) {
        return;
    }
    for item in items.filter(|item| snapped_kind(item)) {
        // Relaxed: a non-finite edge is not asked to be whole. An
        // infinite edge is not between pixels, and the a11y take refuses
        // a box with one (`AxError::NotFinite`); paint does not yet,
        // issue #449. That issue's fix, paint refusing the box, retires
        // this relaxation.
        assert!(
            whole_where_finite(item),
            "box slot {slot}, in a frame of class {:?}, paints {item:?}, not whole device \
             pixels: snapped items under a whole translation are whole (paint.rs \
             `snapped`); reproduce with `cargo xtask test`",
            input.class
        );
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
