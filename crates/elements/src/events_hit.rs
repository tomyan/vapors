//! What the last painted update says is under a point: the element
//! hit, its cursor, and `:hover` following a layout change.

use vapors_css::tree::States;
use vapors_dom::NodeId;

use super::Point;
use crate::document::Document;

impl Document {
    /// The cursor over `point` in the last painted update: the hit
    /// element's, or the default arrow over nothing or before the first.
    #[must_use]
    pub fn cursor(&self, point: Point) -> crate::Cursor {
        self.painted
            .as_ref()
            .map_or(crate::Cursor::Default, |painted| painted.cursor(point))
    }

    /// The element `point` hits in the last painted update, or `None`
    /// before the first, after a refused one, or where the element hit
    /// has been removed since (review-v77, H1).
    #[must_use]
    pub fn hit(&self, point: Point) -> Option<NodeId> {
        self.painted
            .as_ref()?
            .hit(point)
            .filter(|node| self.contains(*node))
    }

    /// After an update: the element under the pointer's last position may
    /// have changed with the layout, so `:hover` follows it (the next
    /// update shows it) without an event. Tested only when a hit the
    /// update changed could move it (`HitsChanged`, M11); else the hover
    /// stands, as it does while an element holds the mouse capture
    /// (`hover_capturing`, review-v222 M2).
    pub(crate) fn rehover_in(&mut self, painted: &crate::painted::Painted) {
        let Some(point) = self.pointer.position else {
            return;
        };
        if self.mouse_captured() {
            return;
        }
        if !painted
            .hits_changed
            .may_move_hover(point, self.pointer.hovered)
        {
            return;
        }
        self.counts.hovers_tested += 1;
        let target = painted.hit(point);
        if target != self.pointer.hovered {
            self.set_flag_chain(States::HOVER, target);
            self.pointer.hovered = target;
        }
    }

    /// Whether an element still in the document holds the mouse
    /// capture, so `:hover` stays on it (`events_capture.rs`).
    fn mouse_captured(&self) -> bool {
        self.captured(super::capture::MOUSE)
            .capture
            .is_some_and(|node| self.contains(node))
    }
}
