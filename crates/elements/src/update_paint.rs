//! An update's paint: the last one kept, or a new one
//! (`kept_paint.rs`).

use crate::changed_slots::ChangedSlots;
use crate::document::Document;
use crate::kept_paint::{PaintedFrom, keep, paint_all};
use crate::layout::{LayoutBox, Rebuilt};
use crate::node_map::NodeMap;
use crate::paint::PaintInput;
use crate::paint_order::{Patched, Stackings};
use crate::painted::Painted;
use crate::style::Used;

impl Document {
    /// Each slot's stacking in `boxes`, for paint order and scroll
    /// chains: `last`'s patched for `rebuilt`, or every one made when
    /// nothing is kept; and whether a rebuilt slot's stacking changed.
    /// A debug build checks them against stackings made afresh.
    pub(crate) fn stackings(
        &mut self,
        boxes: &[LayoutBox],
        rebuilt: &Rebuilt,
        last: Option<&mut Painted>,
    ) -> (Stackings, Patched) {
        let used = &self.kept.as_ref().expect("styled keeps the styles").used;
        let kept = last.filter(|_| !rebuilt.every);
        let (stackings, patched, made) = patched_or_made(kept, boxes, used, rebuilt);
        self.counts.stackings_computed = made;
        if cfg!(debug_assertions) {
            stackings.assert_matches(&Stackings::of(boxes, used));
        }
        (stackings, patched)
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
        let painted = match last {
            Some(kept) if kept.from.shows(from, changed) => {
                keep(kept, boxes, &from.placement.rects, &mut input)
            }
            last => {
                let painted = paint_all(boxes, &from.placement.rects, &mut input, (last, changed));
                self.counts.note_paint(&painted);
                painted
            }
        };
        let scrolls = |slot| stackings.scrolls(slot);
        crate::frames_check::assert_frames_follow_placement(&painted.frames, from, &scrolls);
        painted
    }
}

/// `last`'s stackings patched for `rebuilt`, else every slot's made
/// from `boxes`; what the patch found (every slot restacked, when every
/// one is made); and how many were made.
fn patched_or_made(
    last: Option<&mut Painted>,
    boxes: &[LayoutBox],
    used: &NodeMap<Used>,
    rebuilt: &Rebuilt,
) -> (Stackings, Patched, usize) {
    if let Some(painted) = last {
        let (patched, found) = std::mem::take(&mut painted.stackings).patched(boxes, used, rebuilt);
        return (patched, found, rebuilt.count());
    }
    let made = Stackings::of(boxes, used);
    let held = made.held();
    let every = Patched {
        restacked: true,
        freed_stacked: true,
    };
    (made, every, held)
}
