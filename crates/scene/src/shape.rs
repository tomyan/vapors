//! A shape item (docs/plans/one-scene.md 1.4): an entry of the shape
//! table drawn under an affine transform in one colour. The entry is
//! named by a [`ShapeRef`] the painter resolves to its shape store, as
//! an [`ImageId`](crate::ImageId) is resolved, so this crate depends on
//! no shape crate.

use crate::{Color, Rect};

/// Which shape an item draws, which the painter resolves to an entry
/// it holds (`vapors-app`'s `ShapeStore`): a table entry, by the index
/// the store issued on registration (S5b's process registry will issue
/// `ShapeId`s), or a run-time entry of a document's shape cache (S4a).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ShapeRef {
    /// A table entry.
    Table(u32),
    /// A run-time entry.
    Runtime(RuntimeRef),
}

/// A run-time entry: its document, its slot in that document's cache,
/// and the slot's generation, which goes up each time the slot is
/// freed and reused, so a ref to an entry since freed is refused, never
/// resolved to the shape that took its slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RuntimeRef {
    /// The document whose cache made it.
    pub document: u32,
    /// Its slot in the cache.
    pub index: u32,
    /// The slot's generation when it was made.
    pub generation: u32,
}

impl std::fmt::Display for ShapeRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Table(index) => write!(f, "shape {index}"),
            Self::Runtime(at) => write!(
                f,
                "run-time shape {} (generation {}) of document {}",
                at.index, at.generation, at.document
            ),
        }
    }
}

/// Which pipeline draws an entry: its kind, as `vapors_app::kind_of`
/// gives it from the entry's geometry. Batching splits on it; the
/// painter's store refuses an item whose kind differs from its entry's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShapeKind {
    /// A rect or ellipse, drawn by its closed-form distance.
    ClosedForm,
    /// Triangles from the tessellator.
    Mesh,
}

/// One shape to draw.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shape {
    /// Which entry.
    pub shape: ShapeRef,
    /// Its entry's kind, set once at construction ([`Shape::new`];
    /// `vapors_app::shape_item` takes it from the entry).
    pub(crate) kind: ShapeKind,
    /// From the entry's viewBox units to device pixels, as CSS's
    /// `matrix(a, b, c, d, e, f)`: `x' = a x + c y + e`,
    /// `y' = b x + d y + f`.
    pub transform: [f32; 6],
    /// The fill or stroke colour, non-premultiplied. Whether the entry
    /// is filled or stroked, and how wide, is the entry's (its
    /// canonical paint, `vapors_shapes::Paint`).
    pub colour: Color,
    /// Every pixel the drawn entry covers, device pixels, before the
    /// fringe every item's bounds get: what it is culled by.
    pub bounds: Rect,
}

impl Shape {
    /// An item drawing entry `shape`, of `kind`, under `transform` in
    /// `colour`, culled by `bounds`. Build one from its entry with
    /// `vapors_app::shape_item`, which takes the kind from the entry's
    /// geometry, so the two cannot disagree.
    #[must_use]
    pub fn new(
        shape: ShapeRef,
        kind: ShapeKind,
        transform: [f32; 6],
        colour: Color,
        bounds: Rect,
    ) -> Self {
        Self {
            shape,
            kind,
            transform,
            colour,
            bounds,
        }
    }

    /// Its entry's kind.
    #[must_use]
    pub fn kind(&self) -> ShapeKind {
        self.kind
    }
}

#[cfg(test)]
#[path = "shape_tests.rs"]
mod tests;
