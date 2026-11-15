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
        let Some(painted) = self.painted.as_ref() else {
            return crate::Cursor::Default;
        };
        self.hit(point)
            .map_or(crate::Cursor::Default, |node| self.cursor_of(painted, node))
    }

    /// The element `point` hits in the last painted update, or `None`
    /// before the first, after a refused one, or where the element hit
    /// has been removed since (review-v77, H1). Over a `gpu-surface`
    /// with a watched scene, the scene anchor the scene answers
    /// (`Dom::watch_hits`), or the surface where it answers none.
    #[must_use]
    pub fn hit(&self, point: Point) -> Option<NodeId> {
        let painted = self.painted.as_ref()?;
        let element = painted.hit(point).filter(|node| self.contains(*node))?;
        Some(self.scene_hit(painted, element, point).unwrap_or(element))
    }

    /// After an update: the element under the pointer's last position may
    /// have changed with the layout, so `:hover` follows it (the next
    /// update shows it) without an event. Tested only when a hit the
    /// update changed could move it (`HitsChanged`, M11); else the hover
    /// stands, as it does while an element holds the mouse capture
    /// (`hover_capturing`, review-v222 M2).
    pub(crate) fn rehover_in(&mut self, painted: &crate::painted::Painted) {
        // a watched scene's hits moved under the pointer (`hits_moved`);
        // taken first, so a flag set while the pointer was away is spent
        let moved = std::mem::take(&mut self.scene_hits.moved);
        let Some(point) = self.pointer.position else {
            return;
        };
        if self.mouse_captured() {
            return;
        }
        if !moved
            && !painted
                .hits_changed
                .may_move_hover(point, self.pointer.hovered)
        {
            return;
        }
        self.counts.hovers_tested += 1;
        let element = painted.hit(point).filter(|node| self.contains(*node));
        let target =
            element.map(|element| self.scene_hit(painted, element, point).unwrap_or(element));
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
