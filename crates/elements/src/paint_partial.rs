//! Paint that copies kept paint (IU3 3c-2, `docs/plans/iu3.md`): paint
//! order and clips are walked again, each box kept from the last update
//! with the values its kept paint was made from has that paint copied,
//! ids renumbered in paint order, and every other box is painted again
//! and its paint kept for the next update (`kept_box.rs`). Scroll
//! thumbs are painted on every update. The scene equals a full paint's
//! (`paint.rs`), which a debug build checks (`kept_paint.rs`).

use crate::kept_box::{BoxFrom, KeptBox, Part};
use crate::layout::{BoxRect, LayoutBox};
use crate::paint::{PaintInput, clip_to, order, paint_box, started};
use crate::paint_order::Step;
use crate::painted::Painted;

/// Paints `boxes` at `rects`, copying from `last`, the last update's
/// kept paint by slot, where it shows what painting again would.
pub(crate) fn paint_partial(
    boxes: &[LayoutBox],
    rects: &[BoxRect],
    input: &mut PaintInput<'_>,
    mut last: Vec<Option<KeptBox>>,
) -> Painted {
    let mut painted = started(boxes, rects, input);
    let (steps, clips) = order(boxes, rects, input);
    last.resize_with(boxes.len(), || None);
    let mut walk = Walk {
        boxes,
        rects,
        last,
        kept: std::iter::repeat_with(|| None).take(boxes.len()).collect(),
        scratch: Painted::empty(painted.canvas, painted.canvas_from, input.scale),
    };
    for step in steps {
        let index = step.index();
        clip_to(clips[index], input, &mut painted.scene);
        match step {
            Step::Box(_) => walk.own(index, clips[index], input, &mut painted),
            Step::Thumb(_) => thumb(&boxes[index], rects[index], input, &mut painted),
            Step::Outline(_) => walk.outline(index, &mut painted),
        }
    }
    painted.kept_boxes = walk.kept;
    painted
}

/// A scroll thumb, painted on every update, its items counted.
fn thumb(layout_box: &LayoutBox, rect: BoxRect, input: &PaintInput<'_>, out: &mut Painted) {
    let before = out.scene.items().len();
    crate::thumb::paint(layout_box, rect, input, out);
    out.items_built += out.scene.items().len() - before;
}

struct Walk<'a> {
    boxes: &'a [LayoutBox],
    rects: &'a [BoxRect],
    /// The last update's kept paint, by slot, taken as it is used.
    last: Vec<Option<KeptBox>>,
    /// This update's, by slot.
    kept: Vec<Option<KeptBox>>,
    /// Where a box painted again is painted, before it is kept.
    scratch: Painted,
}

impl Walk<'_> {
    /// Box `index`'s own step: its kept paint copied when it shows what
    /// painting again would, else painted again.
    fn own(&mut self, index: usize, clip: Option<BoxRect>, input: &mut PaintInput<'_>, out: &mut Painted) {
        let from = BoxFrom::of(self.boxes, self.rects, clip, index, input.propagated);
        let last = self.last[index].take();
        let reused = last.filter(|last| self.boxes[index].kept && last.from == from);
        let kept = match reused {
            Some(kept) => {
                kept.touch(input.cache);
                kept
            }
            None => self.painted_again(index, from, input, out),
        };
        kept.own.copy_into(out);
        self.kept[index] = Some(kept);
    }

    /// Box `index`'s own step and outline painted, kept, and counted.
    fn painted_again(
        &mut self,
        index: usize,
        from: BoxFrom,
        input: &mut PaintInput<'_>,
        out: &mut Painted,
    ) -> KeptBox {
        let (layout_box, rect) = (&self.boxes[index], self.rects[index]);
        clip_to(input.clip, input, &mut self.scratch.scene);
        paint_box(self.boxes, self.rects, index, input, &mut self.scratch);
        let own = Part::take(&mut self.scratch);
        clip_to(input.clip, input, &mut self.scratch.scene);
        crate::paint_outline::paint_outline(layout_box, rect, input, &mut self.scratch);
        let outline = std::mem::take(&mut self.scratch.scene).items().to_vec();
        let text = KeptBox::text_key(layout_box, rect, input.scale);
        let kept = KeptBox { from, own, outline, text };
        out.boxes_drawn += 1;
        out.items_built += kept.items();
        kept
    }

    /// Box `index`'s outline, kept at its own step.
    fn outline(&self, index: usize, out: &mut Painted) {
        let kept = self.kept[index]
            .as_ref()
            .expect("a box's outline is painted after its own step");
        for item in &kept.outline {
            out.scene.push(*item);
        }
    }
}
