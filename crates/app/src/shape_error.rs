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
        site: String,
        /// The kind the item says.
        item: ShapeKind,
        /// The kind its entry's geometry is.
        entry: ShapeKind,
    },
    /// A table whose meshes would take the painter's entry buffer past
    /// the device's `max_buffer_size`.
    MeshBytes {
        /// What would add them: a crate's table, or a document's
        /// run-time shapes.
        owner: String,
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
        site: String,
    },
    /// The item's bounds, which culling reads, do not hold what it
    /// draws (`shape_bounds.rs`).
    OutsideBounds {
        /// The ref.
        shape: ShapeRef,
        /// Where it was written.
        site: String,
        /// What it draws, device pixels, with the ramp.
        drawn: Rect,
        /// The item's bounds.
        bounds: Rect,
    },
    /// A document's shapes given to a store that holds another
    /// document's run-time shapes.
    OtherDocument {
        /// The document whose shapes the store holds.
        held: u32,
        /// The document whose shapes it was given.
        given: u32,
    },
    /// A run-time shape the store cannot draw: its geometry or paint.
    Runtime {
        /// Where it was written.
        site: String,
        /// Why.
        reason: String,
    },
    /// The closed-form pipeline refused the item: its transform.
    Undrawable {
        /// The ref.
        shape: ShapeRef,
        /// Where it was written.
        site: String,
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
                "{shape} is not registered in this painter's store, which holds {registered}; register its crate's table (ShapeStore::register) before drawing it, or draw a document's run-time shapes with the painter that draws its frames"
            ),
            Self::Undrawable {
                shape,
                site,
                reason,
            } => write!(f, "{shape}, the entry at {site}, cannot be drawn: {reason}"),
            Self::OutsideBounds {
                shape,
                site,
                drawn,
                bounds,
            } => write!(
                f,
                "{shape}, the entry at {site}, draws over {drawn:?} but its item's bounds are {bounds:?}, so culling would drop pixels it draws; give the item bounds that hold its transformed entry"
            ),
            Self::Kind {
                shape,
                site,
                item,
                entry,
            } => write!(
                f,
                "{shape}, the entry at {site}, is a {entry:?} but its item says {item:?}; build the item's kind from its entry's geometry; reproduce with `cargo test -p vapors-app --lib shape_store`"
            ),
            Self::MeshBytes {
                owner,
                needed,
                limit,
            } => write!(
                f,
                "{owner} would put {needed} bytes of mesh triangles in the painter's entry buffer, over the device's max_buffer_size of {limit}; draw fewer or smaller meshes, or draw some with a second ScenePainter; reproduce with `cargo test -p vapors-app --lib shape_store_meshes`"
            ),
            Self::OtherDocument { held, given } => write!(
                f,
                "this painter holds the run-time shapes of document {held} and was given document {given}'s; draw each document with a ScenePainter of its own; reproduce with `cargo test -p vapors-app --lib shape_store_runtime`"
            ),
            Self::Runtime { site, reason } => write!(
                f,
                "the run-time shape at {site} cannot be drawn: {reason}; this is a fault in the document's shape cache, so report it with the page; reproduce with `cargo test -p vapors-app --lib shape_store_runtime`"
            ),
            Self::NotUploaded { shape, site } => write!(
                f,
                "{shape}, the mesh at {site}, has no triangles on the GPU: their upload this frame was refused, and the frame's encoder holds why; the store refuses a table its buffer cannot hold, so report this with the frame's error; reproduce with `cargo test -p vapors-app --lib shape_store_meshes`"
            ),
        }
    }
}

impl std::error::Error for ShapeError {}
