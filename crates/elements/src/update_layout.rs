//! An update's layout phase: the boxes built again where marked, laid
//! out, and what paint is made from besides them.

use crate::changed_slots::ChangedSlots;
use crate::document::Document;
use crate::error::ElementError;
use crate::kept_paint::PaintedFrom;
#[cfg(debug_assertions)]
use crate::layout::layout_check::{Inputs, check_layout_against_fresh};
use crate::layout::{Key, Placed, Rebuilt, Slots, Written, lay_out};
use crate::paint_order::Stackings;
use crate::painted::Painted;
use crate::placement::{Changes, LaidOut, Placed as Placing, Placement};
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
        let (boxes, mut built) = self.rebuilt(viewport.scale);
        let kept = self.kept.as_ref().expect("styled keeps the styles");
        let size = (viewport.width, viewport.height);
        let texts = (&mut self.fonts, &mut self.texts);
        let changed = std::mem::take(&mut built.changed);
        let laid_out = lay_out(boxes, texts, viewport.scale, size, &kept.calcs, changed);
        let work = match &laid_out {
            Ok((_, work)) => work,
            Err(failed) => &failed.1,
        };
        self.counts.count_layout(work);
        let (slotted, work) = laid_out.map_err(|failed| vec![ElementError::Layout(failed.0)])?;
        // Debug timings include this full build and layout on every
        // narrowed update; `layout_check.rs` says what would retire it.
        // A test whose own oracle compares the same layout each step
        // skips it (`skip_layout_check`).
        #[cfg(debug_assertions)]
        if !built.every && !self.boxes.skip_layout_check {
            self.check_layout(&slotted, viewport);
        }
        Ok((slotted, built, work.written))
    }

    /// Debug builds skip the layout check (`layout/layout_check.rs`)
    /// on this document's updates. Only for a test whose own oracle
    /// compares every update's layout with a full update's; it says why
    /// where it calls this.
    #[doc(hidden)]
    pub fn skip_layout_check(&mut self) {
        #[cfg(debug_assertions)]
        {
            self.boxes.skip_layout_check = true;
        }
    }

    /// Panics when `placed`'s layout is not a fresh layout's
    /// (`layout/layout_check.rs`).
    #[cfg(debug_assertions)]
    fn check_layout(&mut self, placed: &Placed, viewport: Viewport) {
        let root = self.root();
        let text_marks = self.focused_text_marks();
        let kept = self.kept.as_ref().expect("styled keeps the styles");
        let source = crate::layout::Source {
            arena: &self.arena,
            used: &kept.used,
            images: &self.images,
            text_marks,
        };
        let layout = Inputs {
            fonts: &mut self.fonts,
            calcs: &kept.calcs,
            viewport,
        };
        check_layout_against_fresh(placed, &source, root, layout);
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
        let done = self.placed(&mut placement, (&boxes, &placed.slots), (written, changed));
        (changed.moved, changed.offsets_moved) = (done.moved, done.offsets_moved);
        changed.scroll_states = done.states_changed;
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
    /// `written`, and returns what it placed. A debug build checks
    /// the result against a fresh placement (`placement_check.rs`).
    fn placed(
        &mut self,
        placement: &mut Placement,
        (boxes, slots): (&LaidOut<'_>, &Slots),
        (written, changed): (&Written, &ChangedSlots),
    ) -> Placing {
        let scrolled = self.scrolled_slots(slots);
        let since = Changes::since(changed, written, &scrolled);
        let before = cfg!(debug_assertions).then(|| Before::of(&self.scroll, placement));
        let done = placement.place(boxes, &mut self.scroll, &since);
        self.forget_freed(&changed.freed, slots);
        self.counts.boxes_placed = done.count;
        self.counts.boxes_moved = done.moved.len();
        self.counts.slots_chained = done.chained;
        self.check_placed(before, (boxes, placement), (&done, changed));
        done
    }

    /// In a debug build, panics when `placement` and the offsets are not
    /// a fresh placement of `boxes` from `before`, or `moved` is not the
    /// slots that moved (`placement_check.rs`).
    fn check_placed(
        &self,
        before: Option<Before>,
        (boxes, placement): (&LaidOut<'_>, &Placement),
        placed: (&Placing, &ChangedSlots),
    ) {
        if let Some(before) = before {
            let kept = (placement, &self.scroll);
            crate::placement_check::assert_same(&before, boxes, kept, placed);
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
