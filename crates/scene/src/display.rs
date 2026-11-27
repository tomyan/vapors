//! The display list: what the element tree paints and the renderer
//! draws, in painting order. Coordinates are device pixels from the
//! top-left of the target, y down.

use crate::Color;
use crate::clip::Clip;
use crate::shape::Shape;

/// An axis-aligned rectangle in device pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    /// Left edge.
    pub left: f32,
    /// Top edge.
    pub top: f32,
    /// Right edge.
    pub right: f32,
    /// Bottom edge.
    pub bottom: f32,
}

impl Rect {
    /// The rectangle from its edges.
    #[must_use]
    pub fn new(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    /// Width; negative when the edges are crossed.
    #[must_use]
    pub fn width(&self) -> f32 {
        self.right - self.left
    }

    /// Height; negative when the edges are crossed.
    #[must_use]
    pub fn height(&self) -> f32 {
        self.bottom - self.top
    }
}

/// A box: an element's background and solid border, with circular
/// corner radii. The border is drawn inside `rect`; the background
/// fills the whole rect, under the border, as CSS's
/// `background-clip: border-box` paints it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quad {
    /// Where, in device pixels: the border box.
    pub rect: Rect,
    /// The fill, non-premultiplied.
    pub background: Color,
    /// Border widths in device pixels: top, right, bottom, left.
    pub border: [f32; 4],
    /// Border colours: top, right, bottom, left.
    pub border_colors: [Color; 4],
    /// Corner radii of the outer edge in device pixels: top-left,
    /// top-right, bottom-right, bottom-left.
    pub radii: [f32; 4],
}

impl Quad {
    /// A filled rectangle with no border and square corners.
    #[must_use]
    pub fn fill(rect: Rect, background: Color) -> Self {
        Self {
            rect,
            background,
            border: [0.0; 4],
            border_colors: [Color::default(); 4],
            radii: [0.0; 4],
        }
    }
}

/// A box's outer shadow: the shape of `rect` with `radii`, blurred.
/// The painter draws it outside the box only, as CSS does: what lies
/// under the box is the box's to paint.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shadow {
    /// The shadow's shape before blurring: the border box moved by the
    /// offset and grown by the spread, in device pixels.
    pub rect: Rect,
    /// Its corner radii (the box's, grown by the spread).
    pub radii: [f32; 4],
    /// The blur radius in device pixels (CSS's, so the Gaussian's
    /// standard deviation is half of it).
    pub blur: f32,
    /// The colour, non-premultiplied.
    pub color: Color,
    /// The box that casts it, which the shadow is not drawn under.
    pub caster: Rect,
    /// The caster's corner radii.
    pub caster_radii: [f32; 4],
}

/// Names a laid-out text run the painter's owner holds. The scene does
/// not hold text itself, so it needs no dependency on the text crate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TextId(pub u32);

/// Which image an item draws: an index the painter resolves, as with
/// [`TextId`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ImageId(pub u32);

/// Which icon an item draws: an index the painter resolves, as with
/// [`ImageId`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct IconId(pub u32);

/// Which GPU surface an item draws: an index the painter resolves to
/// the surface's name, as with [`ImageId`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SurfaceId(pub u32);

/// One thing to draw.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Item {
    /// A box's outer shadow, drawn before the box.
    Shadow(Shadow),
    /// A filled box.
    Quad(Quad),
    /// An image stretched over `rect` (device pixels), by the id its
    /// painter resolves.
    Image {
        /// Which image.
        image: ImageId,
        /// Where it is drawn.
        rect: Rect,
    },
    /// An `<svg>` icon's paths stroked in `colour`, scaled uniformly
    /// into `rect` (device pixels, whole) and centred, by the id its
    /// painter resolves.
    Icon {
        /// Which icon.
        icon: IconId,
        /// Where it is drawn: the element's content box.
        rect: Rect,
        /// The element's `color`, non-premultiplied.
        colour: Color,
    },
    /// A GPU surface's content over `rect` (device pixels): what its
    /// renderer drew, by the id its painter resolves.
    Surface {
        /// Which surface.
        surface: SurfaceId,
        /// Where it is drawn: the element's content box.
        rect: Rect,
    },
    /// A shape from the shape table (`shape.rs`).
    Shape(Shape),
    /// A text run, its layout origin at `origin` (device pixels).
    Text {
        /// Which run.
        text: TextId,
        /// Top-left of the layout.
        origin: [f32; 2],
        /// Every pixel its glyphs can ink, device pixels: the bounds
        /// the painter culls it by.
        ink: Rect,
    },
}

/// Items in painting order: later items draw over earlier ones, each
/// with the clip that was set when it was pushed. Each item names its
/// clip by its place in the scene's own list of the clips it uses
/// (`scene_clips.rs`), 4 bytes an item where a clip is 32: a kept box
/// holds a scene, and its slot is held to a budget
/// (`vapors-elements` `kept_box_size_tests.rs`).
#[derive(Clone, Debug, Default)]
pub struct Scene {
    pub(crate) items: Vec<Item>,
    /// Each item's clip: 0 for none, else 1 + its place in `palette`.
    pub(crate) clip_of: Vec<u32>,
    /// The distinct clips the items name, in the order first set; none
    /// until a clip is set.
    pub(crate) palette: Option<Box<[Clip]>>,
    /// The clip the next item is pushed under, as `clip_of` names one.
    pub(crate) pen: u32,
}

impl Scene {
    /// An empty scene.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            items: Vec::new(),
            clip_of: Vec::new(),
            palette: None,
            pen: 0,
        }
    }

    /// Appends `item`, over everything before it, under the current
    /// clip.
    pub fn push(&mut self, item: Item) {
        self.items.push(item);
        self.clip_of.push(self.pen);
    }

    /// Clips the items pushed from now on to `clip` (device pixels), a
    /// rectangle, or lifts the clip with `None`.
    pub fn set_clip(&mut self, clip: Option<Rect>) {
        self.set_rounded_clip(clip.map(Clip::square));
    }

    /// Clips the items pushed from now on to `clip` (device pixels), which
    /// may have rounded corners, or lifts the clip with `None`.
    pub fn set_rounded_clip(&mut self, clip: Option<Clip>) {
        self.pen = self.name(clip);
    }

    /// The items, first drawn first.
    #[must_use]
    pub fn items(&self) -> &[Item] {
        &self.items
    }

    /// The items, to change in place; their clips stay.
    pub fn items_mut(&mut self) -> &mut [Item] {
        &mut self.items
    }
}
