//! An update's layout phase: the boxes built again where marked, laid
//! out, and what paint is made from besides them.

use crate::document::Document;
use crate::error::ElementError;
use crate::kept_paint::PaintedFrom;
use crate::layout::{LayoutBox, Placed, lay_out, placed};
use crate::paint_order::Stackings;
use crate::update::Viewport;

impl Document {
    /// The boxes, built again where marked and laid out for
    /// `viewport`, and how many were built.
    pub(crate) fn laid_out(
        &mut self,
        viewport: Viewport,
    ) -> Result<(Placed, usize), Vec<ElementError>> {
        let (boxes, built) = self.rebuilt(viewport.scale);
        let kept = self.kept.as_ref().expect("styled keeps the styles");
        let size = (viewport.width, viewport.height);
        let texts = (&mut self.fonts, &mut self.texts);
        let laid_out = lay_out(boxes, texts, viewport.scale, size, &kept.calcs);
        let (Ok((_, work)) | Err((_, work))) = &laid_out;
        (self.counts.boxes_laid_out, self.counts.layout_calls) = (work.laid_out, work.calls);
        let (slotted, _) = laid_out.map_err(|(failure, _)| vec![ElementError::Layout(failure)])?;
        Ok((slotted, built))
    }

    /// The boxes built again where marked, the rest kept, and how many
    /// were built.
    fn rebuilt(&mut self, scale: f32) -> (Placed, usize) {
        let root = self.root();
        let text_marks = self.focused_text_marks();
        let kept = self.kept.as_ref().expect("styled keeps the styles");
        kept.calcs.begin();
        let source = crate::layout::Source {
            arena: &self.arena,
            used: &kept.used,
            images: &self.images,
            text_marks,
        };
        let may_keep = |node| kept.may_keep_layout(node);
        let (boxes, built) = self.boxes.rebuild(&source, root, scale, &may_keep);
        self.counts.boxes_built = built;
        debug_assert!(
            !boxes.boxes.is_empty() || kept.used[&root].layout.display == taffy::Display::None,
            "no boxes built while html is shown: a box build dropped the root"
        );
        (boxes, built)
    }

    /// What paint is made from besides `boxes`: their rects, scroll
    /// offsets settled and applied, the thumbs and the scale.
    pub(crate) fn painted_from(
        &mut self,
        boxes: &[LayoutBox],
        stackings: &Stackings,
        viewport: Viewport,
    ) -> PaintedFrom {
        let offsets = self
            .scroll
            .settle(boxes, &|index| stackings.scrolls(index), viewport.scale);
        PaintedFrom {
            rects: placed(boxes, &offsets),
            thumbs_shown: self.thumbs_shown,
            scale: viewport.scale,
        }
    }
}
