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
        let Some(painted) = &self.painted else {
            return false;
        };
        let Some(element) = painted.hit(point) else {
            return false;
        };
        let chain = painted.scroll_chains.from(element);
        let moved = chain
            .into_iter()
            .any(|container| self.scroll.scroll(container, delta));
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

    /// The scroll offset of `element` (CSS pixels, unrounded) as of the
    /// last update and the scrolls since, or `None` when it is not a
    /// scroll container.
    #[must_use]
    pub fn scroll_offset(&self, element: NodeId) -> Option<[f32; 2]> {
        self.scroll.get(element)
    }
}
