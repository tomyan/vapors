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
    /// A table whose meshes would take the painter's entry buffer past
    /// the device's `max_buffer_size`.
    MeshBytes {
        /// The table's crate.
        crate_name: &'static str,
        /// The entry buffer's bytes with the table.
        needed: u64,
        /// The device's `max_buffer_size`.
        limit: u64,
    },
    /// A mesh entry whose triangles are not on the GPU: its upload at
    /// the start of the frame was refused (on the frame's encoder).
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
            Self::MeshBytes {
                crate_name,
                needed,
                limit,
            } => write!(
                f,
                "shape table of crate {crate_name} would put {needed} bytes of mesh triangles in the painter's entry buffer, over the device's max_buffer_size of {limit}; register fewer or smaller mesh tables, or draw some with a second ScenePainter; reproduce with `cargo test -p vapors-app --lib shape_store_meshes`"
            ),
            Self::NotUploaded { shape, site } => write!(
                f,
                "shape {}, the mesh at {site}, has no triangles on the GPU: their upload this frame was refused, and the frame's encoder holds why; the store refuses a table its buffer cannot hold, so report this with the frame's error; reproduce with `cargo test -p vapors-app --lib shape_store_meshes`",
                shape.0
            ),
        }
    }
}

impl std::error::Error for ShapeError {}
