//! The shapes a painter draws (docs/plans/one-scene.md 1.4): each
//! shape table registered gives its entries the [`ShapeRef`]s the
//! scene's shape items name, in order after those registered before.
//! A closed form is drawn from its entry's geometry and the paint its
//! canonical bytes begin with; its parameters go in each instance's
//! bytes, so it uploads nothing (`shape_bytes_uploaded` is 0 for closed
//! forms). A mesh entry is registered and refused at draw
//! until the mesh pipeline (S3a2). The process registry (S5b) will
//! register each crate's table once, on its first mount.

use std::fmt;

use vapors_renderer::{ClosedFormError, ClosedFormShape};
use vapors_scene::{Shape, ShapeRef};
use vapors_shapes::{Geometry, Paint, ShapeEntry, ShapeTable, TableError, paint_of};

/// The registered entries, by [`ShapeRef`].
#[derive(Debug, Default)]
pub struct ShapeStore {
    entries: Vec<Registered>,
}

#[derive(Debug)]
struct Registered {
    entry: &'static ShapeEntry,
    /// The stroke width its paint names; `None` fills it.
    stroke: Option<f32>,
}

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

impl ShapeStore {
    /// Registers `table`'s entries, after its check; each entry's ref,
    /// in the table's order.
    ///
    /// # Errors
    /// [`ShapeError::Table`] when the table's check refuses it (an
    /// undrawable geometry, a rect with elliptical corners, bytes that
    /// begin with no paint, among others); nothing is registered then.
    pub fn register(&mut self, table: &ShapeTable) -> Result<Vec<ShapeRef>, ShapeError> {
        table.check().map_err(ShapeError::Table)?;
        let first = self.entries.len();
        self.entries.extend(table.entries().iter().map(registered));
        Ok((first..self.entries.len()).map(shape_ref).collect())
    }

    /// `shapes` as the closed-form pipeline draws them: each entry with
    /// the item's transform and colour; an item whose transform has no
    /// area (CSS's `scale(0)`) is left out, as it draws nothing.
    ///
    /// # Errors
    /// For the first that fails: [`ShapeError::Unregistered`] for a ref
    /// the store did not issue, [`ShapeError::Mesh`] for a mesh entry,
    /// [`ShapeError::Undrawable`] for a transform the pipeline refuses.
    pub fn closed_forms(&self, shapes: &[Shape]) -> Result<Vec<ClosedFormShape>, ShapeError> {
        let mut drawn = Vec::with_capacity(shapes.len());
        for shape in shapes {
            drawn.extend(self.closed_form(shape)?);
        }
        Ok(drawn)
    }

    fn closed_form(&self, item: &Shape) -> Result<Option<ClosedFormShape>, ShapeError> {
        let shape = item.shape;
        let Registered { entry, stroke } =
            self.entries
                .get(shape.0 as usize)
                .ok_or(ShapeError::Unregistered {
                    shape,
                    registered: self.entries.len(),
                })?;
        let site = entry.site;
        if let Geometry::Mesh { .. } = entry.geometry {
            return Err(ShapeError::Mesh { shape, site });
        }
        ClosedFormShape::new(&entry.geometry, *stroke, item.transform, item.colour).map_err(
            |reason| ShapeError::Undrawable {
                shape,
                site,
                reason,
            },
        )
    }
}

fn shape_ref(index: usize) -> ShapeRef {
    ShapeRef(u32::try_from(index).expect("fewer than 2^32 shapes are registered"))
}

/// `entry`, checked by its table, and the stroke width its paint names.
fn registered(entry: &'static ShapeEntry) -> Registered {
    let paint = paint_of(entry.canonical).expect("the table's check read this paint");
    let stroke = match paint {
        Paint::Fill(_) => None,
        Paint::Stroke { width } => Some(width),
    };
    Registered { entry, stroke }
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
            Self::Mesh { shape, site } => write!(
                f,
                "shape {}, the mesh at {site}, is not drawn until the mesh pipeline (one-scene S3a2); draw it as a closed form or wait for S3a2",
                shape.0
            ),
        }
    }
}

impl std::error::Error for ShapeError {}

#[cfg(test)]
#[path = "shape_store_tests.rs"]
mod tests;
