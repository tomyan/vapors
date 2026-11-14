//! Why a shape table was not registered or a shape item not drawn
//! (`shape_store.rs`), each naming the shape and, where it has one, the
//! site its entry was written at.

use std::fmt;

use vapors_renderer::ClosedFormError;
use vapors_scene::{Rect, ShapeRef};
use vapors_shapes::TableError;

/// Why a table was not registered or a shape not drawn.
#[derive(Clone, Debug, PartialEq)]
pub enum ShapeError {
    /// The table's own check refused it.
    Table(TableError),
    /// A ref the store has issued no entry for.
    Unregistered {
        /// The ref.
        shape: ShapeRef,
        /// How many entries the store holds.
        registered: usize,
    },
    /// A mesh entry, which the mesh pipeline (S3a2) will draw.
    Mesh {
        /// The ref.
        shape: ShapeRef,
        /// Where it was written.
        site: &'static str,
    },
    /// The item's bounds, which culling reads, do not hold what it
    /// draws (`shape_bounds.rs`).
    OutsideBounds {
        /// The ref.
        shape: ShapeRef,
        /// Where it was written.
        site: &'static str,
        /// What it draws, device pixels, with the ramp.
        drawn: Rect,
        /// The item's bounds.
        bounds: Rect,
    },
    /// The closed-form pipeline refused the item: its transform.
    Undrawable {
        /// The ref.
        shape: ShapeRef,
        /// Where it was written.
        site: &'static str,
        /// Why.
        reason: ClosedFormError,
    },
}

impl fmt::Display for ShapeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Table(error) => error.fmt(f),
            Self::Unregistered { shape, registered } => write!(
                f,
                "shape {} is not registered in this painter's store, which holds {registered}; register its crate's table (ShapeStore::register) before drawing it",
                shape.0
            ),
            Self::Undrawable {
                shape,
                site,
                reason,
            } => write!(
                f,
                "shape {}, the entry at {site}, cannot be drawn: {reason}",
                shape.0
            ),
            Self::OutsideBounds {
                shape,
                site,
                drawn,
                bounds,
            } => write!(
                f,
                "shape {}, the entry at {site}, draws over {drawn:?} but its item's bounds are {bounds:?}, so culling would drop pixels it draws; give the item bounds that hold its transformed entry",
                shape.0
            ),
            Self::Mesh { shape, site } => write!(
                f,
                "shape {}, the mesh at {site}, is not drawn until the mesh pipeline (one-scene S3a2); draw it as a closed form or wait for S3a2",
                shape.0
            ),
        }
    }
}

impl std::error::Error for ShapeError {}
