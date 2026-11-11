//! What one box's paint is written into before it is kept as its part
//! (`kept_box.rs`): the scene items, the ids they name in the paint's
//! slabs (`slab.rs`), and the box's entries for the pointer and focus.

use vapors_dom::NodeId;
use vapors_scene::Scene;

use crate::layout::BoxRect;
use crate::node_map::{NodeMap, NodeSet};
use crate::style::Cursor;

/// One step's paint, being written.
#[derive(Default)]
pub(crate) struct Painting {
    pub(crate) scene: Scene,
    /// The paint's slabs, lent while a box is painted.
    pub(crate) slabs: crate::slab::Slabs,
    /// The parts of its box that take the pointer, in painting order.
    pub(crate) hits: Vec<(NodeId, BoxRect)>,
    pub(crate) cursors: NodeMap<Cursor>,
    pub(crate) visible: NodeSet,
    /// Its inline elements' border boxes (`paint_inline.rs`).
    pub(crate) rects: NodeMap<BoxRect>,
    pub(crate) ranges: NodeMap<crate::range_paint::Track>,
    pub(crate) text_origins: NodeMap<[f32; 2]>,
}
