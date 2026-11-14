//! Why a shape table was not registered or a shape item not drawn
//! (`shape_store.rs`), each naming the shape and, where it has one, the
//! site its entry was written at.

use std::fmt;

use vapors_renderer::ClosedFormError;
use vapors_scene::{Rect, ShapeKind, ShapeRef};
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
    /// An item whose kind is not its entry's: batched for one
    /// pipeline, its entry needs the other.
    Kind {
        /// The ref.
        shape: ShapeRef,
        /// Where it was written.
        site: &'static str,
        /// The kind the item says.
        item: ShapeKind,
        /// The kind its entry's geometry is.
        entry: ShapeKind,
    },
    /// A mesh entry whose vertices are not on the GPU: the painter
    /// uploads every registered entry at the start of each frame.
    NotUploaded {
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
            Self::Kind {
                shape,
                site,
                item,
                entry,
            } => write!(
                f,
                "shape {}, the entry at {site}, is a {entry:?} but its item says {item:?}; build the item's kind from its entry's geometry; reproduce with `cargo test -p vapors-app --lib shape_store`",
                shape.0
            ),
            Self::NotUploaded { shape, site } => write!(
                f,
                "shape {}, the mesh at {site}, has no vertices on the GPU; draw through ScenePainter, which uploads every registered entry before a frame; reproduce with `cargo test -p vapors-app --test gpu_shapes`",
                shape.0
            ),
        }
    }
}

impl std::error::Error for ShapeError {}
