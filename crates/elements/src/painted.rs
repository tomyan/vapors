//! The result of an update: what was painted, where each element is,
//! and which elements take the pointer, in painting order.

use std::collections::HashMap;

use vapors_dom::NodeId;
use vapors_scene::{Color, Scene, TextId};
use vapors_text::Layout;

use crate::layout::BoxRect;
use crate::style::Cursor;

/// The canvas colour, the scene, the text runs it names, each element's
/// border box, and the elements a pointer can hit.
pub struct Painted {
    pub(crate) canvas: Color,
    pub(crate) scene: Scene,
    pub(crate) texts: Vec<std::rc::Rc<Layout>>,
    pub(crate) rects: HashMap<NodeId, BoxRect>,
    /// Elements with a box, `visibility: visible` and `pointer-events:
    /// auto`, in painting order: a later one is drawn over an earlier.
    pub(crate) hits: Vec<(NodeId, BoxRect)>,
    /// The cursor over each element in `hits`.
    pub(crate) cursors: HashMap<NodeId, Cursor>,
    /// Which scroll containers a scroll over each element tries.
    pub(crate) scroll_chains: crate::scroll_chains::Chains,
}

impl Painted {
    /// The colour under everything: what the window is cleared to.
    #[must_use]
    pub fn canvas(&self) -> Color {
        self.canvas
    }

    /// The display list.
    #[must_use]
    pub fn scene(&self) -> &Scene {
        &self.scene
    }

    /// The text run a scene item names.
    ///
    /// # Panics
    /// When `id` is not from this update's scene.
    #[must_use]
    pub fn text(&self, id: TextId) -> &Layout {
        &self.texts[id.0 as usize]
    }

    /// The element's border box in CSS pixels, or `None` when it has no
    /// box (`display: none`, or outside the document).
    #[must_use]
    pub fn border_box(&self, node: NodeId) -> Option<BoxRect> {
        self.rects.get(&node).copied()
    }

    /// The cursor over `point`: the one of the element it hits, the
    /// default arrow where it hits none.
    #[must_use]
    pub fn cursor(&self, point: [f32; 2]) -> Cursor {
        self.hit(point)
            .and_then(|node| self.cursors.get(&node).copied())
            .unwrap_or(Cursor::Default)
    }

    /// The topmost element under `point` (CSS pixels) that takes the
    /// pointer: the last one painted whose border box holds it. A box
    /// holds its left and top edges, not its right and bottom.
    #[must_use]
    pub fn hit(&self, point: [f32; 2]) -> Option<NodeId> {
        let [x, y] = point;
        self.hits
            .iter()
            .rev()
            .find(|(_, r)| x >= r.x && x < r.x + r.width && y >= r.y && y < r.y + r.height)
            .map(|(node, _)| *node)
    }
}
