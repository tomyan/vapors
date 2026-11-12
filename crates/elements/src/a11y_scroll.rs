//! Scrolling a node into view before a press (R1), as Chromium's
//! accessibility default action does (`ScrollRectToVisible` with
//! `ScrollAlignment::CenterIfNeeded`): each scroll container around the
//! node, nearest first, centres it on an axis where it is not wholly
//! in view, as far as the container can go. The viewport itself does
//! not scroll in vapors, so a node outside it stays outside.

use vapors_dom::NodeId;

use crate::clip::padding_box;
use crate::document::Document;
use crate::layout::BoxRect;

impl Document {
    /// Scrolls the containers around `node`, whose border box is
    /// `bounds` in the last update, to bring it into view; whether one
    /// moved (the next update shows it).
    pub(crate) fn scroll_into_view(&mut self, node: NodeId, bounds: BoxRect) -> bool {
        let Some(painted) = self.painted.as_ref() else {
            return false;
        };
        let chain = painted.order.chains.from(node);
        let mut rect = bounds;
        let mut moved = false;
        for container in chain.into_iter().filter(|c| *c != node) {
            let shift = self.centre_in(container, rect);
            moved |= shift != [0.0, 0.0];
            rect.x -= shift[0];
            rect.y -= shift[1];
        }
        moved
    }

    /// Scrolls `container` to centre `rect` on each axis where its
    /// scrollport does not hold it wholly; how far the content moved.
    fn centre_in(&mut self, container: NodeId, rect: BoxRect) -> [f32; 2] {
        let Some(port) = self.scrollport(container) else {
            return [0.0, 0.0];
        };
        let delta = [
            centring(rect.x, rect.width, port.x, port.width),
            centring(rect.y, rect.height, port.y, port.height),
        ];
        let before = self.scroll.get(container).unwrap_or_default();
        self.scroll_node_by(container, delta);
        let after = self.scroll.get(container).unwrap_or_default();
        [after[0] - before[0], after[1] - before[1]]
    }

    /// The padding box of `container` in the last update: where its
    /// scrolled content shows.
    fn scrollport(&self, container: NodeId) -> Option<BoxRect> {
        let rect = *self.painted.as_ref()?.rects.get(&container)?;
        Some(padding_box(self.boxes.element(container)?, rect))
    }
}

/// The scroll along one axis that centres `start..start + length` in
/// `port_start..port_start + port_length`, or none when it is in it.
fn centring(start: f32, length: f32, port_start: f32, port_length: f32) -> f32 {
    if start >= port_start && start + length <= port_start + port_length {
        return 0.0;
    }
    (start + length / 2.0) - (port_start + port_length / 2.0)
}
