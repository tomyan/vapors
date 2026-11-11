//! An update's paint: the last one kept, or a new one
//! (`kept_paint.rs`).

use std::collections::HashMap;

use vapors_dom::NodeId;

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
        let used = &self.kept.as_ref().expect("styled keeps the styles").used;
        let scrolls = scroll_containers(boxes, used);
        let mut input = PaintInput {
            used,
            root: self.root(),
            body: self.body(),
            propagated: None,
            fonts: &mut self.fonts,
            cache: &mut self.texts,
            scale: from.scale,
            clip: None,
            scroll: &self.scroll,
            thumbs_shown: from.thumbs_shown,
        };
        if let Some(kept) = last.filter(|last| last.from.shows(from, built)) {
            return keep(kept, boxes, &from.rects, &mut input, &scrolls);
        }
        let painted = paint_all(boxes, &from.rects, &mut input, &scrolls);
        self.counts.note_paint(&painted);
        painted
    }
}

/// Whether the box at an index is a scroll container.
pub(crate) fn scroll_containers<'a>(
    boxes: &'a [LayoutBox],
    used: &'a HashMap<NodeId, Used>,
) -> impl Fn(usize) -> bool + 'a {
    move |index| match boxes[index].content {
        Content::Element(node) => used[&node].scrolls(),
        Content::Text(_) => false,
        Content::Vacant => unreachable!("a vacant slot is no box's child"),
    }
}
