//! The shapes a painter draws (docs/plans/one-scene.md 1.4): each
//! shape table registered gives its entries the [`ShapeRef`]s the
//! scene's shape items name, in order after those registered before.
//! A closed form is drawn from its entry's geometry and the paint its
//! canonical bytes begin with; its parameters go in each instance's
//! bytes, so it uploads nothing (`shape_bytes_uploaded` is 0 for closed
//! forms). A mesh entry's triangles are uploaded once, at the start of
//! the first frame after it is registered (`shape_store_meshes.rs`).
//! The process registry (S5b) will register each crate's table once,
//! on its first mount.

use vapors_renderer::{ClosedFormShape, MeshEntry};
use vapors_scene::{Shape, ShapeKind, ShapeRef};

use crate::shape_bounds::outside_bounds;
pub use crate::shape_error::ShapeError;
use vapors_shapes::{Geometry, Paint, ShapeEntry, ShapeTable, paint_of};

/// The registered entries, by [`ShapeRef`].
#[derive(Debug)]
pub struct ShapeStore {
    entries: Vec<Registered>,
    /// The first entry whose triangles, if a mesh, are not uploaded.
    pub(crate) unuploaded: usize,
    /// The entry buffer's bytes for every mesh registered.
    pub(crate) mesh_bytes: u64,
    /// The most the entry buffer can hold: the device's
    /// `max_buffer_size`, which registration refuses to pass.
    pub(crate) buffer_limit: u64,
}

impl ShapeStore {
    /// A store whose entry buffer holds at most `limit` bytes (the
    /// device's `max_buffer_size`).
    #[must_use]
    pub fn with_buffer_limit(limit: u64) -> Self {
        Self {
            entries: Vec::new(),
            unuploaded: 0,
            mesh_bytes: 0,
            buffer_limit: limit,
        }
    }
}

impl Default for ShapeStore {
    /// A store with no limit on its entry buffer, for tests without a
    /// device; the painter's has the device's.
    fn default() -> Self {
        Self::with_buffer_limit(u64::MAX)
    }
}

#[derive(Debug)]
pub(crate) struct Registered {
    pub(crate) entry: &'static ShapeEntry,
    /// The stroke width its paint names; `None` fills it.
    stroke: Option<f32>,
    /// A mesh's triangles on the GPU, once uploaded.
    pub(crate) uploaded: Option<MeshEntry>,
}

impl ShapeStore {
    /// Registers `table`'s entries, after its check; each entry's ref,
    /// in the table's order.
    ///
    /// # Errors
    /// [`ShapeError::Table`] when the table's check refuses it (an
    /// undrawable geometry, a rect with elliptical corners, bytes that
    /// begin with no paint, among others), and
    /// [`ShapeError::MeshBytes`] when its meshes would take the entry
    /// buffer past the device's `max_buffer_size`; nothing is
    /// registered then.
    pub fn register(&mut self, table: &ShapeTable) -> Result<Vec<ShapeRef>, ShapeError> {
        table.check().map_err(ShapeError::Table)?;
        self.mesh_bytes = self.check_mesh_bytes(table)?;
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
    /// the store did not issue, [`ShapeError::Kind`] for an item whose
    /// entry is a mesh, [`ShapeError::Undrawable`] for a transform the
    /// pipeline refuses, [`ShapeError::OutsideBounds`] for bounds that
    /// do not hold what the item draws.
    pub fn closed_forms(&self, shapes: &[Shape]) -> Result<Vec<ClosedFormShape>, ShapeError> {
        let mut drawn = Vec::with_capacity(shapes.len());
        for shape in shapes {
            drawn.extend(self.closed_form(shape)?);
        }
        Ok(drawn)
    }

    fn closed_form(&self, item: &Shape) -> Result<Option<ClosedFormShape>, ShapeError> {
        let Registered { entry, stroke, .. } = self.of_kind(item, ShapeKind::ClosedForm)?;
        let drawn = ClosedFormShape::new(&entry.geometry, *stroke, item.transform, item.colour)
            .map_err(|reason| undrawable(item, entry, reason))?;
        if drawn.is_some() {
            check_bounds(entry, *stroke, item)?;
        }
        Ok(drawn)
    }

    /// The entry `item` names, when it is registered and of the kind
    /// both the item and the run it is drawn in say.
    pub(crate) fn of_kind(&self, item: &Shape, run: ShapeKind) -> Result<&Registered, ShapeError> {
        let shape = item.shape;
        let registered = self
            .entries
            .get(shape.0 as usize)
            .ok_or(ShapeError::Unregistered {
                shape,
                registered: self.entries.len(),
            })?;
        let entry = kind_of(&registered.entry.geometry);
        if item.kind() != entry || run != entry {
            return Err(ShapeError::Kind {
                shape,
                site: registered.entry.site,
                item: item.kind(),
                entry,
            });
        }
        Ok(registered)
    }

    /// The entry at `index`, which the store issued.
    pub(crate) fn at(&self, index: usize) -> &Registered {
        &self.entries[index]
    }

    pub(crate) fn entries_mut(&mut self) -> &mut [Registered] {
        &mut self.entries
    }
}

/// An item drawing `entry`, registered as `shape`, with its kind from
/// the entry's geometry ([`kind_of`]): the way to build a shape item,
/// so its kind and its entry's cannot disagree.
#[must_use]
pub fn shape_item(
    entry: &ShapeEntry,
    shape: ShapeRef,
    transform: [f32; 6],
    colour: vapors_scene::Color,
    bounds: vapors_scene::Rect,
) -> Shape {
    Shape::new(shape, kind_of(&entry.geometry), transform, colour, bounds)
}

/// The kind `geometry` is drawn as: the one answer an item's kind is
/// held to.
#[must_use]
pub fn kind_of(geometry: &Geometry) -> ShapeKind {
    match geometry {
        Geometry::Rect { .. } | Geometry::Ellipse { .. } => ShapeKind::ClosedForm,
        Geometry::Mesh { .. } => ShapeKind::Mesh,
    }
}

pub(crate) fn undrawable(
    item: &Shape,
    entry: &ShapeEntry,
    reason: vapors_renderer::ClosedFormError,
) -> ShapeError {
    ShapeError::Undrawable {
        shape: item.shape,
        site: entry.site,
        reason,
    }
}

/// Refuses `item` when what `entry` draws under it lies outside its
/// bounds, which culling reads.
pub(crate) fn check_bounds(
    entry: &ShapeEntry,
    stroke: Option<f32>,
    item: &Shape,
) -> Result<(), ShapeError> {
    match outside_bounds(&entry.geometry, stroke, item) {
        None => Ok(()),
        Some(drawn) => Err(ShapeError::OutsideBounds {
            shape: item.shape,
            site: entry.site,
            drawn,
            bounds: item.bounds,
        }),
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
    Registered {
        entry,
        stroke,
        uploaded: None,
    }
}

#[cfg(test)]
#[path = "shape_store_tests.rs"]
pub(crate) mod tests;
