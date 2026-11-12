//! Where a paint put each element, on the page: its entries are kept in
//! its scroll frame and put on the page as they are read (`frames.rs`).

use vapors_dom::NodeId;

use crate::layout::BoxRect;
use crate::painted::Painted;
use crate::range_paint::Track;

impl Painted {
    /// The element's border box in CSS pixels, or `None` when it has no
    /// box (`display: none`, or outside the document).
    #[must_use]
    pub fn border_box(&self, node: NodeId) -> Option<BoxRect> {
        let rect = *self.rects.get(&node)?;
        Some(self.frames.rect(self.frame_of(node), rect))
    }

    /// The element's padding box's top-left in CSS pixels, or `None`
    /// when it has no box.
    #[must_use]
    pub fn padding_origin(&self, node: NodeId) -> Option<[f32; 2]> {
        let origin = *self.paddings.get(&node)?;
        Some(self.frames.point(self.frame_of(node), origin))
    }

    /// A text field's text origin, CSS pixels.
    pub(crate) fn text_origin(&self, node: NodeId) -> Option<[f32; 2]> {
        let origin = *self.text_origins.get(&node)?;
        Some(self.frames.point(self.frame_of(node), origin))
    }

    /// A range input's track.
    pub(crate) fn range_track(&self, node: NodeId) -> Option<Track> {
        let track = *self.ranges.get(&node)?;
        let [x, _] = self.frames.point(self.frame_of(node), [track.x, 0.0]);
        Some(Track { x, ..track })
    }

    /// The scroll frame `node`'s entries are in, or a panic naming it.
    fn frame_of(&self, node: NodeId) -> usize {
        *self.node_frames.get(&node).unwrap_or_else(|| {
            panic!(
                "{node:?} has a painted position but no scroll frame: its entry was put \
                 without one (kept_maps.rs `put`, kept_box.rs `enter`); reproduce with \
                 `cargo xtask test`"
            )
        })
    }
}

#[cfg(test)]
#[path = "painted_positions_tests.rs"]
mod tests;
