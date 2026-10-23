//! The display list: what the element tree paints and the renderer
//! draws, in painting order. Coordinates are device pixels from the
//! top-left of the target, y down.

use crate::Color;

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

/// One thing to draw.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Item {
    /// A box's outer shadow, drawn before the box.
    Shadow(Shadow),
    /// A filled box.
    Quad(Quad),
    /// A text run, its layout origin at `origin` (device pixels).
    Text {
        /// Which run.
        text: TextId,
        /// Top-left of the layout.
        origin: [f32; 2],
    },
}

/// Items in painting order: later items draw over earlier ones.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Scene {
    items: Vec<Item>,
}

impl Scene {
    /// An empty scene.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends `item`, over everything before it.
    pub fn push(&mut self, item: Item) {
        self.items.push(item);
    }

    /// The items, first drawn first.
    #[must_use]
    pub fn items(&self) -> &[Item] {
        &self.items
    }

    /// Runs of consecutive items of one kind, in order: what a painter
    /// draws with one pipeline before switching to the next.
    #[must_use]
    pub fn batches(&self) -> Vec<Batch<'_>> {
        let mut batches: Vec<Batch<'_>> = Vec::new();
        let mut start = 0;
        for end in 1..=self.items.len() {
            if end == self.items.len() || kind(&self.items[end]) != kind(&self.items[start]) {
                batches.push(Batch::of(&self.items[start..end]));
                start = end;
            }
        }
        batches
    }
}

/// A run of items one pipeline draws.
#[derive(Clone, Debug, PartialEq)]
pub enum Batch<'a> {
    /// Consecutive shadows.
    Shadows(Vec<Shadow>),
    /// Consecutive quads.
    Quads(Vec<Quad>),
    /// Consecutive text runs.
    Texts(&'a [Item]),
}

impl<'a> Batch<'a> {
    fn of(items: &'a [Item]) -> Self {
        match items[0] {
            Item::Shadow(_) => Batch::Shadows(
                items
                    .iter()
                    .filter_map(|item| match item {
                        Item::Shadow(shadow) => Some(*shadow),
                        _ => None,
                    })
                    .collect(),
            ),
            Item::Quad(_) => Batch::Quads(
                items
                    .iter()
                    .filter_map(|item| match item {
                        Item::Quad(quad) => Some(*quad),
                        _ => None,
                    })
                    .collect(),
            ),
            Item::Text { .. } => Batch::Texts(items),
        }
    }
}

fn kind(item: &Item) -> u8 {
    match item {
        Item::Shadow(_) => 0,
        Item::Quad(_) => 1,
        Item::Text { .. } => 2,
    }
}

#[cfg(test)]
#[path = "display_tests.rs"]
mod tests;
