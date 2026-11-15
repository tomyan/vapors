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
        let chain = painted.order.chains.from(element);
        chain
            .into_iter()
            .find(|container| self.scroll.can_move(*container, delta))
    }

    /// Scrolls `container` alone by `delta`, never passing it out;
    /// whether it moved.
    pub fn scroll_node_by(&mut self, container: NodeId, delta: [f32; 2]) -> bool {
        let moved = self.scroll.scroll(container, delta);
        if moved {
            self.note_scrolled(container);
        }
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
        if moved {
            self.note_scrolled(element);
        }
        moved
    }

    /// Notes `container` scrolled: a change, and the a11y mirror reads
    /// the container's offset at its next take.
    fn note_scrolled(&mut self, container: NodeId) {
        self.note_change();
        self.ax.touch_node(container);
    }

    /// Shows or hides every scroll container's thumbs from the next
    /// update.
    pub fn set_thumbs_shown(&mut self, shown: bool) {
        self.set_thumb_opacity(if shown { 1.0 } else { 0.0 });
    }

    /// Draws every scroll container's thumbs at `opacity` (0 hidden, 1
    /// as styled; clamped, quantised to 256 steps) from the next update:
    /// a touch app shows them while scrolling and fades them after.
    pub fn set_thumb_opacity(&mut self, opacity: f32) {
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "clamped to 0..=1 first, so 0..=255"
        )]
        let alpha = (opacity.clamp(0.0, 1.0) * f32::from(u8::MAX)).round() as u8;
        if self.thumb_alpha != alpha {
            self.note_change();
        }
        self.thumb_alpha = alpha;
    }

    /// The scroll offset of `element` (CSS pixels, unrounded) as of the
    /// last update and the scrolls since, or `None` when it is not a
    /// scroll container.
    #[must_use]
    pub fn scroll_offset(&self, element: NodeId) -> Option<[f32; 2]> {
        self.scroll.get(element)
    }
}
