//! A shape item (docs/plans/one-scene.md 1.4): an entry of the shape
//! table drawn under an affine transform in one colour. The entry is
//! named by a [`ShapeRef`] the painter resolves to its shape store, as
//! an [`ImageId`](crate::ImageId) is resolved, so this crate depends on
//! no shape crate.

use crate::{Color, Rect};

/// Which shape an item draws: an index the painter resolves to an
/// entry it has registered (`vapors-app`'s `ShapeStore`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ShapeRef(pub u32);

/// One shape to draw.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shape {
    /// Which entry.
    pub shape: ShapeRef,
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
