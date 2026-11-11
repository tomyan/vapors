//! An update's layout phase: the boxes built again where marked, laid
//! out, and what paint is made from besides them.

use crate::changed_slots::ChangedSlots;
use crate::document::Document;
use crate::error::ElementError;
use crate::kept_paint::PaintedFrom;
use crate::layout::{Key, Placed, Rebuilt, Slots, Written, lay_out};
use crate::paint_order::Stackings;
use crate::painted::Painted;
use crate::placement::{Changes, LaidOut, Placement};
use crate::placement_check::Before;
use crate::update::Viewport;

impl Document {
    /// The boxes, built again where marked and laid out for
    /// `viewport`, the slots built and freed, and the slots layout
    /// laid out or wrote.
    pub(crate) fn laid_out(
        &mut self,
        viewport: Viewport,
    ) -> Result<(Placed, Rebuilt, Written), Vec<ElementError>> {
        let (boxes, built) = self.rebuilt(viewport.scale);
        let kept = self.kept.as_ref().expect("styled keeps the styles");
        let size = (viewport.width, viewport.height);
        let texts = (&mut self.fonts, &mut self.texts);
        let laid_out = lay_out(boxes, texts, viewport.scale, size, &kept.calcs);
        let (Ok((_, work)) | Err((_, work))) = &laid_out;
        (self.counts.boxes_laid_out, self.counts.layout_calls) = (work.laid_out, work.calls);
        let (slotted, work) =
            laid_out.map_err(|(failure, _)| vec![ElementError::Layout(failure)])?;
        Ok((slotted, built, work.written))
    }

    /// The boxes built again where marked, the rest kept, and the
    /// slots built and freed.
    fn rebuilt(&mut self, scale: f32) -> (Placed, Rebuilt) {
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
        self.counts.boxes_built = built.count();
        debug_assert!(
            !boxes.boxes.is_empty() || kept.used[&root].layout.display == taffy::Display::None,
            "no boxes built while html is shown: a box build dropped the root"
        );
        (boxes, built)
    }

    /// What paint is made from besides `placed`'s boxes: the
    /// placement kept in `last`, patched for what layout `written` and
    /// `changed` name (`placement.rs`), which notes the moved slots in
    /// `changed`; the thumbs and the scale.
    pub(crate) fn painted_from(
        &mut self,
        (placed, stackings): (&Placed, &Stackings),
        viewport: Viewport,
        (last, written): (Option<&mut Painted>, &Written),
        changed: &mut ChangedSlots,
    ) -> PaintedFrom {
        let take = |last: &mut Painted| std::mem::take(&mut last.from.placement);
        let mut placement = last.map(take).unwrap_or_default();
        let scrolls = |slot| stackings.scrolls(slot);
        let boxes = LaidOut::new(&placed.boxes, &scrolls, viewport.scale);
        let moved = self.placed(&mut placement, (&boxes, &placed.slots), (written, changed));
        changed.moved = moved;
        self.made_from(placement, viewport.scale)
    }

    /// What paint is made from: `placement`, the thumbs and `scale`.
    fn made_from(&self, placement: Placement, scale: f32) -> PaintedFrom {
        PaintedFrom {
            placement,
            thumbs_shown: self.thumbs_shown,
            scale,
        }
    }

    /// Places `boxes` in `placement` for `changed` and what layout
    /// `written`, and returns the moved slots. A debug build checks
    /// the result against a fresh placement (`placement_check.rs`).
    fn placed(
        &mut self,
        placement: &mut Placement,
        (boxes, slots): (&LaidOut<'_>, &Slots),
        (written, changed): (&Written, &ChangedSlots),
    ) -> Vec<usize> {
        let scrolled = self.scrolled_slots(slots);
        let since = Changes::since(changed, written, &scrolled);
        let before = cfg!(debug_assertions).then(|| Before::of(&self.scroll, placement));
        let done = placement.place(boxes, &mut self.scroll, &since);
        self.forget_freed(&changed.freed, slots);
        (self.counts.boxes_placed, self.counts.boxes_moved) = (done.count, done.moved.len());
        self.check_placed(before, (boxes, placement), (&done.moved, changed));
        done.moved
    }

    /// In a debug build, panics when `placement` and the offsets are not
    /// a fresh placement of `boxes` from `before`, or `moved` is not the
    /// slots that moved (`placement_check.rs`).
    fn check_placed(
        &self,
        before: Option<Before>,
        (boxes, placement): (&LaidOut<'_>, &Placement),
        moved: (&[usize], &ChangedSlots),
    ) {
        if let Some(before) = before {
            let kept = (placement, &self.scroll);
            crate::placement_check::assert_same(&before, boxes, kept, moved);
        }
    }

    /// The slots of the containers scrolled since the last update.
    fn scrolled_slots(&mut self, slots: &Slots) -> Vec<usize> {
        let scrolled = self.scroll.take_scrolled();
        scrolled
            .into_iter()
            .filter_map(|node| slots.of(Key::Element(node)))
            .collect()
    }

    /// Drops the offsets of the freed elements that hold no box now.
    fn forget_freed(&mut self, freed: &[(usize, Key)], slots: &Slots) {
        for &(_, key) in freed {
            if let Key::Element(node) = key
                && slots.of(key).is_none()
            {
                self.scroll.forget(node);
            }
        }
    }
}
