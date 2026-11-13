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
    /// Each element's border box, in its scroll frame (`frames.rs`).
    pub(crate) rects: NodeMap<BoxRect>,
    /// The scroll frame of each element with an entry in `rects`,
    /// `paddings`, `ranges` or `text_origins`, whose positions are in it.
    pub(crate) node_frames: NodeMap<usize>,
    /// Each scroll frame's shift and clip on the page.
    pub(crate) frames: crate::frames::Frames,
    /// Each element's padding box's top-left, for pointer offsets.
    pub(crate) paddings: NodeMap<[f32; 2]>,
    /// The elements drawn: with a box and `visibility: visible` (focus,
    /// 3-E2a).
    pub(crate) visible: NodeSet,
    /// Each part's hits in painting order, gathered from the scene view
    /// on the first hit test after paint order is walked, then patched
    /// by each update (`kept_hits.rs`).
    pub(crate) hits: std::cell::OnceCell<crate::kept_hits::KeptHits>,
    /// The hits this update removed or added, when known, for the
    /// hover after it (`events_hit.rs`).
    pub(crate) hits_changed: crate::kept_hits::HitsChanged,
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
    /// The steps of `order` that draw, which the scene's items walk.
    pub(crate) drawn: crate::drawn_steps::DrawnSteps,
    /// Each scroll frame's drawing steps by their extents, which the
    /// kept batches find the steps in a view through.
    pub(crate) extents: crate::frame_extents::Extents,
    /// The last draw's batches and what they were made from.
    pub(crate) kept_batches: std::cell::RefCell<Option<crate::kept_batches::KeptBatches>>,
    /// Boxes painted by this update, for `UpdateCounts`.
    pub(crate) boxes_drawn: usize,
    /// Boxes whose kept paint this update shifted in place
    /// (`kept_shift.rs`), for `UpdateCounts`.
    pub(crate) boxes_shifted: usize,
    /// Kept parts this update visited without painting again (`paint_partial.rs`), for
    /// `UpdateCounts`.
    pub(crate) parts_visited_kept: usize,
    /// Scene items this update built, for `UpdateCounts`.
    pub(crate) items_built: usize,
    /// Elements whose `rects` and `paddings` entries this update put or
    /// removed (`kept_maps.rs`), for `UpdateCounts`.
    pub(crate) maps_patched: usize,
    /// Entries the patches of `drawn` and `extents` moved (`blocks.rs`).
    pub(crate) index_entries_moved: usize,
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
            node_frames: NodeMap::default(),
            frames: crate::frames::Frames::page(scale),
            paddings: NodeMap::default(),
            visible: NodeSet::default(),
            hits: std::cell::OnceCell::new(),
            hits_changed: crate::kept_hits::HitsChanged::Unknown,
            cursors: NodeMap::default(),
            order: crate::kept_order::KeptOrder::default(),
            ranges: NodeMap::default(),
            text_origins: NodeMap::default(),
            from: crate::kept_paint::PaintedFrom::default(),
            kept_boxes: Vec::new(),
            drawn: crate::drawn_steps::DrawnSteps::default(),
            extents: crate::frame_extents::Extents::default(),
            kept_batches: std::cell::RefCell::new(None),
            boxes_drawn: 0,
            boxes_shifted: 0,
            parts_visited_kept: 0,
            items_built: 0,
            maps_patched: 0,
            index_entries_moved: 0,
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
        let parts = (&self.kept_boxes[..], &self.frames);
        crate::SceneView::new(&self.order.steps, &self.drawn, parts)
    }

    /// The text run a scene item names.
    ///
    /// # Panics
    /// When `id` is not one this update's scene names.
    #[must_use]
    pub fn text(&self, id: TextId) -> &std::rc::Rc<Layout> {
        resolved(self.slabs.texts.get(id.0), "text", id.0)
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
    pub fn icon(&self, id: vapors_scene::IconId) -> &std::rc::Rc<vapors_text::Icon> {
        resolved(self.slabs.icons.get(id.0), "icon", id.0)
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

    /// Whether the element was drawn: it has a box and is
    /// `visibility: visible`.
    #[must_use]
    pub fn is_visible(&self, node: NodeId) -> bool {
        self.visible.contains(&node)
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
        self.kept_hits().hit([x, y], &self.frames)
    }

    /// The kept hits, gathered from the scene view when none are kept.
    pub(crate) fn kept_hits(&self) -> &crate::kept_hits::KeptHits {
        self.hits
            .get_or_init(|| crate::kept_hits::KeptHits::of(&self.scene()))
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
