//! An update's paint: the last one kept, or a new one
//! (`kept_paint.rs`).

use crate::node_map::NodeMap;

use crate::document::Document;
use crate::kept_paint::{PaintedFrom, keep, paint_all};
use crate::layout::{Content, LayoutBox};
use crate::paint::PaintInput;
use crate::painted::Painted;
use crate::style::Used;

impl Document {
    /// `last` when it shows what a paint made from `from` would after
    /// an update that built `built` boxes, else a new paint of `boxes`.
    pub(crate) fn paint_or_keep(
        &mut self,
        last: Option<Painted>,
        boxes: &[LayoutBox],
        from: &PaintedFrom,
        built: usize,
    ) -> Painted {
        let page = (self.root(), self.body());
        let used = &self.kept.as_ref().expect("styled keeps the styles").used;
        let scrolls = scroll_containers(boxes, used);
        let text = (&mut self.fonts, &mut self.texts);
        let mut input = PaintInput::new(used, page, text, &self.scroll, from);
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

/// Whether the box at an index is a scroll container.
pub(crate) fn scroll_containers<'a>(
    boxes: &'a [LayoutBox],
    used: &'a NodeMap<Used>,
) -> impl Fn(usize) -> bool + 'a {
    move |index| match boxes[index].content {
        Content::Element(node) => used[&node].scrolls(),
        Content::Text(_) => false,
        Content::Vacant => unreachable!("a vacant slot is no box's child"),
    }
}
