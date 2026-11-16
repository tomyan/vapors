//! Shared by the scene hit tests: the scene page's points, and reading
//! what a point hits.

use vapors_dom::{Dom, NodeId};
use vapors_elements::Viewport;

use super::Rig;

pub const VIEWPORT: Viewport = Viewport {
    width: 64.0,
    height: 64.0,
    scale: 1.0,
};

/// The surface's centre: over the left mesh, 1 unit wide at the origin
/// (pixels 24 to 40 at 16 pixels a unit).
pub const CENTRE: [f32; 2] = [32.0, 32.0];
/// Over the right mesh at x 1, when shown (pixels 40 to 56).
pub const RIGHT: [f32; 2] = [48.0, 32.0];
/// Over neither.
pub const CORNER: [f32; 2] = [4.0, 4.0];

impl Rig {
    pub fn updated(&self) {
        self.doc.borrow_mut().update(VIEWPORT).expect("no refusal");
    }

    pub fn hit(&self, point: [f32; 2]) -> Option<NodeId> {
        self.doc.borrow().hit(point)
    }

    /// The tag and class of what `point` hits.
    pub fn hit_names(&self, point: [f32; 2]) -> (String, String) {
        let node = self.hit(point).expect("a hit");
        let doc = self.doc.borrow();
        let tag = doc.tag(node).unwrap_or_default().to_owned();
        let class = doc
            .current_attribute(node, "class")
            .unwrap_or_default()
            .to_owned();
        (tag, class)
    }
}
