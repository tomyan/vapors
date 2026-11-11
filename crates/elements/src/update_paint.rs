//! An update's paint: the last one kept, or a new one
//! (`kept_paint.rs`).

use crate::document::Document;
use crate::kept_paint::{PaintedFrom, keep, paint_all};
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
    /// an update that built `built` boxes, else a new paint of `boxes`.
    pub(crate) fn paint_or_keep(
        &mut self,
        last: Option<Painted>,
        (boxes, stackings): (&[LayoutBox], &Stackings),
        from: &PaintedFrom,
        built: usize,
    ) -> Painted {
        let page = (self.root(), self.body());
        let used = &self.kept.as_ref().expect("styled keeps the styles").used;
        let scrolls = |index| stackings.scrolls(index);
        let text = (&mut self.fonts, &mut self.texts);
        let mut input = PaintInput::new((used, stackings), page, text, &self.scroll, from);
        let last = match last {
            Some(kept) if kept.from.shows(from, built) => {
                return keep(kept, boxes, &from.rects, &mut input, &scrolls);
            }
            Some(last) => last.kept_boxes,
            None => Vec::new(),
        };
        let painted = paint_all(boxes, &from.rects, &mut input, &scrolls, last);
        self.counts.note_paint(&painted);
        painted
    }
}
