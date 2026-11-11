//! The result of an update: what was painted, where each element is,
//! and which elements take the pointer, in painting order.

use crate::node_map::{NodeMap, NodeSet};

use vapors_dom::NodeId;
use vapors_scene::{Color, TextId};
use vapors_text::Layout;

use crate::canvas::CanvasFrom;
use crate::layout::BoxRect;
use crate::style::Cursor;

/// The canvas colour, the scene, the text runs it names, each element's
/// border box, and the elements a pointer can hit.
pub struct Painted {
    pub(crate) canvas: Color,
    /// The element whose background became the canvas's, if one did.
    pub(crate) canvas_from: Option<CanvasFrom>,
    /// Device pixels per CSS pixel: the update's viewport scale.
    pub(crate) scale: f32,
    /// The texts, images, icons and surface names the scene's items
    /// name, by id (`slab.rs`).
    pub(crate) slabs: crate::slab::Slabs,
    pub(crate) rects: NodeMap<BoxRect>,
    /// Each element's padding box's top-left, for pointer offsets.
    pub(crate) paddings: NodeMap<[f32; 2]>,
    /// The elements drawn: with a box and `visibility: visible` (focus,
    /// 3-E2a).
    pub(crate) visible: NodeSet,
    /// Each part's hits in painting order, gathered from the scene view
    /// on the first hit test after the update: one walk of paint order,
    /// then each test reads only the hits.
    pub(crate) hits: std::cell::OnceCell<Vec<(NodeId, BoxRect)>>,
    /// The cursor over each element that takes the pointer.
    pub(crate) cursors: NodeMap<Cursor>,
    /// Paint order, clips and which scroll containers a scroll over
    /// each element tries (`kept_order.rs`).
    pub(crate) order: crate::kept_order::KeptOrder,
    /// Each range input's track, for the pointer (3-E3a).
    pub(crate) ranges: NodeMap<crate::range_paint::Track>,
    /// Each text field's text origin, for the pointer (E6a).
    pub(crate) text_origins: NodeMap<[f32; 2]>,
    /// What this was painted from (`kept_paint`).
    pub(crate) from: crate::kept_paint::PaintedFrom,
    /// Each slot's kept paint (`kept_box.rs`), walked in `order` for the
    /// scene and hits (`scene_view.rs`), kept in place for the next update.
    pub(crate) kept_boxes: Vec<Option<crate::kept_box::KeptBox>>,
    /// Boxes painted by this update, for `UpdateCounts`.
    pub(crate) boxes_drawn: usize,
    /// Kept parts this update read and kept (`paint_partial.rs`), for
    /// `UpdateCounts`.
    pub(crate) parts_read_kept: usize,
    /// Scene items this update built, for `UpdateCounts`.
    pub(crate) items_built: usize,
    /// Elements whose `rects` and `paddings` entries this update put or
    /// removed (`kept_maps.rs`), for `UpdateCounts`.
    pub(crate) maps_patched: usize,
    /// Each slot's stacking, for the next update's shape comparison
    /// (`changed_slots.rs`).
    pub(crate) stackings: crate::paint_order::Stackings,
}

impl Painted {
    /// A paint of nothing on `canvas` at `scale`.
    pub(crate) fn empty(canvas: Color, canvas_from: Option<CanvasFrom>, scale: f32) -> Self {
        Self {
            canvas,
            canvas_from,
            scale,
            slabs: crate::slab::Slabs::default(),
            rects: NodeMap::default(),
            paddings: NodeMap::default(),
            visible: NodeSet::default(),
            hits: std::cell::OnceCell::new(),
            cursors: NodeMap::default(),
            order: crate::kept_order::KeptOrder::default(),
            ranges: NodeMap::default(),
            text_origins: NodeMap::default(),
            from: crate::kept_paint::PaintedFrom::default(),
            kept_boxes: Vec::new(),
            boxes_drawn: 0,
            parts_read_kept: 0,
            items_built: 0,
            maps_patched: 0,
            stackings: crate::paint_order::Stackings::default(),
        }
    }

    /// The colour under everything: what the window is cleared to.
    #[must_use]
    pub fn canvas(&self) -> Color {
        self.canvas
    }

    /// The element whose background is the canvas's, when the root or
    /// the body sets one; `None` when the canvas is the default white.
    #[must_use]
    pub fn canvas_from(&self) -> Option<CanvasFrom> {
        self.canvas_from
    }

    /// The update's viewport scale, device pixels per CSS pixel.
    #[must_use]
    pub fn scale(&self) -> f32 {
        self.scale
    }

    /// The display list: a view over the kept paint in paint order.
    #[must_use]
    pub fn scene(&self) -> crate::SceneView<'_> {
        crate::SceneView::new(&self.order.steps, &self.kept_boxes)
    }

    /// The text run a scene item names.
    ///
    /// # Panics
    /// When `id` is not one this update's scene names.
    #[must_use]
    pub fn text(&self, id: TextId) -> &Layout {
        resolved::<std::rc::Rc<Layout>>(self.slabs.texts.get(id.0), "text", id.0)
    }

    /// The image a scene item names.
    ///
    /// # Panics
    /// When `id` is not one this update's scene names.
    #[must_use]
    pub fn image(&self, id: vapors_scene::ImageId) -> &std::rc::Rc<vapors_scene::RgbaImage> {
        resolved(self.slabs.images.get(id.0), "image", id.0)
    }

    /// The icon a scene item names.
    ///
    /// # Panics
    /// When `id` is not one this update's scene names.
    #[must_use]
    pub fn icon(&self, id: vapors_scene::IconId) -> &vapors_text::Icon {
        resolved::<std::rc::Rc<vapors_text::Icon>>(self.slabs.icons.get(id.0), "icon", id.0)
    }

    /// The name of the surface a scene item names: its `gpu-surface`'s
    /// `name`, which the painter resolves to a mounted renderer.
    ///
    /// # Panics
    /// When `id` is not one this update's scene names.
    #[must_use]
    pub fn surface(&self, id: vapors_scene::SurfaceId) -> &str {
        resolved::<String>(self.slabs.surfaces.get(id.0), "surface", id.0)
    }

    /// The element's border box in CSS pixels, or `None` when it has no
    /// box (`display: none`, or outside the document).
    #[must_use]
    pub fn border_box(&self, node: NodeId) -> Option<BoxRect> {
        self.rects.get(&node).copied()
    }

    /// Whether the element was drawn: it has a box and is
    /// `visibility: visible`.
    #[must_use]
    pub fn is_visible(&self, node: NodeId) -> bool {
        self.visible.contains(&node)
    }

    /// The element's padding box's top-left in CSS pixels, or `None`
    /// when it has no box.
    #[must_use]
    pub fn padding_origin(&self, node: NodeId) -> Option<[f32; 2]> {
        self.paddings.get(&node).copied()
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
    pub fn hit(&self, [x, y]: [f32; 2]) -> Option<NodeId> {
        let hits = self
            .hits
            .get_or_init(|| self.scene().hits().copied().collect());
        hits.iter()
            .rev()
            .find(|(_, r)| x >= r.x && x < r.x + r.width && y >= r.y && y < r.y + r.height)
            .map(|(node, _)| *node)
    }
}

/// The value an id names, or a panic naming the id.
fn resolved<'a, T>(value: Option<&'a T>, kind: &str, id: u32) -> &'a T {
    value.unwrap_or_else(|| {
        panic!(
            "{kind} id {id} is free or was never issued (painted.rs, slab.rs); `cargo xtask test`"
        )
    })
}
