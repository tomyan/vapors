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
    /// The element whose background became the canvas's, if one did.
    pub(crate) canvas_from: Option<CanvasFrom>,
    /// Device pixels per CSS pixel: the update's viewport scale.
    pub(crate) scale: f32,
    pub(crate) scene: Scene,
    pub(crate) texts: Vec<std::rc::Rc<Layout>>,
    pub(crate) rects: HashMap<NodeId, BoxRect>,
    /// Each element's padding box's top-left, for pointer offsets.
    pub(crate) paddings: HashMap<NodeId, [f32; 2]>,
    /// The elements drawn: with a box and `visibility: visible` (focus,
    /// 3-E2a).
    pub(crate) visible: std::collections::HashSet<NodeId>,
    /// Elements with a box, `visibility: visible` and `pointer-events:
    /// auto`, in painting order: a later one is drawn over an earlier.
    pub(crate) hits: Vec<(NodeId, BoxRect)>,
    /// The cursor over each element in `hits`.
    pub(crate) cursors: HashMap<NodeId, Cursor>,
    /// The images the scene's image items name, by `ImageId`.
    pub(crate) images: Vec<std::rc::Rc<vapors_scene::RgbaImage>>,
    /// The icons the scene's icon items name, by `IconId`.
    pub(crate) icons: Vec<std::rc::Rc<vapors_text::Icon>>,
    /// The names of the surfaces the scene's surface items name, by
    /// `SurfaceId`.
    pub(crate) surfaces: Vec<String>,
    /// Which scroll containers a scroll over each element tries.
    pub(crate) scroll_chains: crate::scroll_chains::Chains,
    /// Each range input's track, for the pointer (3-E3a).
    pub(crate) ranges: HashMap<NodeId, crate::range_paint::Track>,
    /// Each text field's text origin, for the pointer (E6a).
    pub(crate) text_origins: HashMap<NodeId, [f32; 2]>,
    /// What this was painted from (`kept_paint`).
    pub(crate) from: crate::kept_paint::PaintedFrom,
    /// Boxes drawn, for `UpdateCounts`.
    pub(crate) boxes_drawn: usize,
}

/// Which element's background became the canvas's (CSS Backgrounds 3
/// section 2.11.2): it then paints none of its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CanvasFrom {
    /// The root (`html`).
    Root,
    /// The body, the root's background being transparent.
    Body,
}

impl CanvasFrom {
    /// The element's tag: `html` or `body`.
    #[must_use]
    pub fn tag(self) -> &'static str {
        match self {
            Self::Root => "html",
            Self::Body => "body",
        }
    }
}

impl Painted {
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

    /// The image a scene item names.
    ///
    /// # Panics
    /// When `id` is not from this update's scene.
    #[must_use]
    pub fn image(&self, id: vapors_scene::ImageId) -> &std::rc::Rc<vapors_scene::RgbaImage> {
        &self.images[id.0 as usize]
    }

    /// The icon a scene item names.
    ///
    /// # Panics
    /// When `id` is not from this update's scene.
    #[must_use]
    pub fn icon(&self, id: vapors_scene::IconId) -> &vapors_text::Icon {
        &self.icons[id.0 as usize]
    }

    /// The name of the surface a scene item names: its `gpu-surface`'s
    /// `name`, which the painter resolves to a mounted renderer.
    ///
    /// # Panics
    /// When `id` is not from this update's scene.
    #[must_use]
    pub fn surface(&self, id: vapors_scene::SurfaceId) -> &str {
        &self.surfaces[id.0 as usize]
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
    pub fn hit(&self, point: [f32; 2]) -> Option<NodeId> {
        let [x, y] = point;
        self.hits
            .iter()
            .rev()
            .find(|(_, r)| x >= r.x && x < r.x + r.width && y >= r.y && y < r.y + r.height)
            .map(|(node, _)| *node)
    }
}
