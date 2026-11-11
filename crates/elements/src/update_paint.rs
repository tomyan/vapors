//! An update's paint: the last one kept, or a new one
//! (`kept_paint.rs`).

use crate::changed_slots::ChangedSlots;
use crate::document::Document;
use crate::kept_paint::{LastPaint, PaintedFrom, keep, paint_all};
use crate::layout::LayoutBox;
use crate::paint::PaintInput;
use crate::paint_order::Stackings;
use crate::painted::Painted;

impl Document {
    /// Each slot's stacking in `boxes`, for paint order and scroll chains.
    pub(crate) fn stackings(&self, boxes: &[LayoutBox]) -> Stackings {
        let used = &self.kept.as_ref().expect("styled keeps the styles").used;
        Stackings::of(boxes, used)
    }

    /// `last` when it shows what a paint made from `from` would after
    /// an update that changed the `changed` slots, else a new paint of
    /// `boxes`.
    pub(crate) fn paint_or_keep(
        &mut self,
        last: Option<Painted>,
        (boxes, stackings): (&[LayoutBox], &Stackings),
        from: &PaintedFrom,
        changed: &ChangedSlots,
    ) -> Painted {
        let page = (self.root(), self.body());
        let used = &self.kept.as_ref().expect("styled keeps the styles").used;
        let text = (&mut self.fonts, &mut self.texts);
        let mut input = PaintInput::new((used, stackings), page, text, &self.scroll, from);
        let last = match last {
            Some(kept) if kept.from.shows(from, changed.built()) => {
                return keep(kept, boxes, &from.rects, &mut input);
            }
            last => LastPaint::of(last),
        };
        let painted = paint_all(boxes, &from.rects, &mut input, (last, changed));
        self.counts.note_paint(&painted);
        painted
    }
}
