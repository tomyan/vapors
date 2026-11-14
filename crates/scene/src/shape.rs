//! A shape item (docs/plans/one-scene.md 1.4): an entry of the shape
//! table drawn under an affine transform in one colour. The entry is
//! named by a [`ShapeRef`] the painter resolves to its shape store, as
//! an [`ImageId`](crate::ImageId) is resolved, so this crate depends on
//! no shape crate.

use crate::{Color, Rect};

/// Which shape an item draws: an index the painter resolves to an
/// entry it has registered (`vapors-app`'s `ShapeStore`). Two ranges:
/// a table entry's, which the store issues on registration (and S5b's
/// process registry will, as `ShapeId`s), and a run-time entry's, an
/// index into its document's shape cache with [`ShapeRef::RUNTIME`]
/// set, which the store takes from the document's frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ShapeRef(pub u32);

impl ShapeRef {
    /// The bit that marks a run-time entry's ref.
    pub const RUNTIME: u32 = 1 << 31;

    /// The ref of the run-time entry at `index` in its document's
    /// cache.
    ///
    /// # Panics
    /// When `index` reaches [`ShapeRef::RUNTIME`]: no document caches
    /// 2^31 shapes.
    #[must_use]
    pub fn runtime(index: u32) -> Self {
        assert!(
            index < Self::RUNTIME,
            "run-time shape {index} is past the run-time range"
        );
        Self(index | Self::RUNTIME)
    }

    /// The index in its document's cache, for a run-time entry's ref.
    #[must_use]
    pub fn runtime_index(self) -> Option<u32> {
        (self.0 & Self::RUNTIME != 0).then_some(self.0 & !Self::RUNTIME)
    }
}

impl std::fmt::Display for ShapeRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.runtime_index() {
            Some(index) => write!(f, "run-time shape {index}"),
            None => write!(f, "shape {}", self.0),
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
