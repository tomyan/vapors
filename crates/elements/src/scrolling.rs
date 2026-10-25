//! Scrolling a document (F2c S9): what wheel and touch input call.

use vapors_dom::NodeId;

use crate::document::Document;

impl Document {
    /// Scrolls by `delta` (CSS pixels; positive moves the content up and
    /// left) the innermost scroll container under `point` in the last
    /// update that can still move that way, or the next one out.
    /// Whether one moved; the next update shows it. Nothing moves before
    /// the first update.
    pub fn scroll_by(&mut self, point: [f32; 2], delta: [f32; 2]) -> bool {
        self.scroll_target(point, delta)
            .is_some_and(|container| self.scroll_node_by(container, delta))
    }

    /// The scroll container [`Document::scroll_by`] would move: the
    /// innermost under `point` that can move by `delta`, else the next
    /// one out. A touch gesture keeps the first one it moves.
    #[must_use]
    pub fn scroll_target(&self, point: [f32; 2], delta: [f32; 2]) -> Option<NodeId> {
        let painted = self.painted.as_ref()?;
        let element = painted.hit(point)?;
        let chain = painted.scroll_chains.from(element);
        chain
            .into_iter()
            .find(|container| self.scroll.can_move(*container, delta))
    }

    /// Scrolls `container` alone by `delta`, never passing it out;
    /// whether it moved.
    pub fn scroll_node_by(&mut self, container: NodeId, delta: [f32; 2]) -> bool {
        let moved = self.scroll.scroll(container, delta);
        self.changed |= moved;
        moved
    }

    /// Scrolls `element` to `offset` (CSS pixels), within its range as of
    /// the last update. Whether it moved; false when it is not a scroll
    /// container.
    pub fn scroll_to(&mut self, element: NodeId, offset: [f32; 2]) -> bool {
        let Some(at) = self.scroll.get(element) else {
            return false;
        };
        let moved = self
            .scroll
            .scroll(element, [offset[0] - at[0], offset[1] - at[1]]);
        self.changed |= moved;
        moved
    }

    /// Shows or hides every scroll container's thumbs from the next
    /// update: a touch app shows them only while a gesture scrolls.
    pub fn set_thumbs_shown(&mut self, shown: bool) {
        self.changed |= self.thumbs_shown != shown;
        self.thumbs_shown = shown;
    }

    /// The scroll offset of `element` (CSS pixels, unrounded) as of the
    /// last update and the scrolls since, or `None` when it is not a
    /// scroll container.
    #[must_use]
    pub fn scroll_offset(&self, element: NodeId) -> Option<[f32; 2]> {
        self.scroll.get(element)
    }
}
