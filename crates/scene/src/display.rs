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

/// A filled box: an element's background. Borders and corner radii
/// join it in F2c S3.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quad {
    /// Where, in device pixels.
    pub rect: Rect,
    /// The fill, non-premultiplied.
    pub background: Color,
}

/// Names a laid-out text run the painter's owner holds. The scene does
/// not hold text itself, so it needs no dependency on the text crate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TextId(pub u32);

/// One thing to draw.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Item {
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
    /// Consecutive quads.
    Quads(Vec<Quad>),
    /// Consecutive text runs.
    Texts(&'a [Item]),
}

impl<'a> Batch<'a> {
    fn of(items: &'a [Item]) -> Self {
        match items[0] {
            Item::Quad(_) => Batch::Quads(
                items
                    .iter()
                    .filter_map(|item| match item {
                        Item::Quad(quad) => Some(*quad),
                        Item::Text { .. } => None,
                    })
                    .collect(),
            ),
            Item::Text { .. } => Batch::Texts(items),
        }
    }
}

fn kind(item: &Item) -> u8 {
    match item {
        Item::Quad(_) => 0,
        Item::Text { .. } => 1,
    }
}

#[cfg(test)]
#[path = "display_tests.rs"]
mod tests;
