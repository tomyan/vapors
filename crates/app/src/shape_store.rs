//! The shapes a painter draws (docs/plans/one-scene.md 1.4): each
//! shape table registered gives its entries the [`ShapeRef`]s the
//! scene's shape items name, in order after those registered before.
//! A closed form keeps its geometry and the paint its canonical bytes
//! begin with; its parameters go in each instance's bytes, so it
//! uploads nothing. A mesh entry is registered and refused at draw
//! until the mesh pipeline (S3a2). The process registry (S5b) will
//! register each crate's table once, on its first mount.

use std::fmt;

use vapors_renderer::{ClosedForm, ClosedFormShape};
use vapors_scene::{Shape, ShapeRef};
use vapors_shapes::{Geometry, Paint, ShapeEntry, ShapeTable, TableError, paint_of};

/// The registered entries, by [`ShapeRef`].
#[derive(Debug, Default)]
pub struct ShapeStore {
    entries: Vec<Registered>,
}

#[derive(Debug)]
struct Registered {
    site: &'static str,
    form: Form,
}

#[derive(Debug)]
enum Form {
    Closed(ClosedForm, Option<f32>),
    Mesh,
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
        let registered = table.entries().iter().map(registered);
        let first = self.entries.len();
        self.entries.extend(registered);
        Ok((first..self.entries.len()).map(shape_ref).collect())
    }

    /// The closed form `shape` names, and its stroke width (`None` to
    /// fill).
    ///
    /// # Errors
    /// [`ShapeError::Unregistered`] for a ref the store did not issue;
    /// [`ShapeError::Mesh`] for a mesh entry.
    pub fn closed_form(&self, shape: ShapeRef) -> Result<(ClosedForm, Option<f32>), ShapeError> {
        let registered = self
            .entries
            .get(shape.0 as usize)
            .ok_or(ShapeError::Unregistered {
                shape,
                registered: self.entries.len(),
            })?;
        match registered.form {
            Form::Closed(form, stroke) => Ok((form, stroke)),
            Form::Mesh => Err(ShapeError::Mesh {
                shape,
                site: registered.site,
            }),
        }
    }
}

impl ShapeStore {
    /// `shapes` as the closed-form pipeline draws them: each entry's
    /// form and stroke with the item's transform and colour.
    ///
    /// # Errors
    /// As [`ShapeStore::closed_form`], for the first that fails.
    pub(crate) fn closed_forms(
        &self,
        shapes: &[Shape],
    ) -> Result<Vec<ClosedFormShape>, ShapeError> {
        shapes
            .iter()
            .map(|shape| {
                let (form, stroke) = self.closed_form(shape.shape)?;
                Ok(ClosedFormShape {
                    form,
                    stroke,
                    transform: shape.transform,
                    colour: shape.colour,
                })
            })
            .collect()
    }
}

fn shape_ref(index: usize) -> ShapeRef {
    ShapeRef(u32::try_from(index).expect("fewer than 2^32 shapes are registered"))
}

/// `entry`, checked by its table: its form and the paint its bytes
/// begin with.
fn registered(entry: &ShapeEntry) -> Registered {
    let paint = paint_of(entry.canonical).expect("the table's check read this paint");
    let stroke = match paint {
        Paint::Fill(_) => None,
        Paint::Stroke { width } => Some(width),
    };
    let form = match entry.geometry {
        Geometry::Rect {
            origin,
            size,
            radius,
        } => Form::Closed(
            ClosedForm::Rect {
                origin,
                size,
                // The table's check refuses radii that differ.
                radius: radius[0],
            },
            stroke,
        ),
        Geometry::Ellipse { centre, radii } => {
            Form::Closed(ClosedForm::Ellipse { centre, radii }, stroke)
        }
        Geometry::Mesh { .. } => Form::Mesh,
    };
    Registered {
        site: entry.site,
        form,
    }
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
