//! The shapes a painter draws (docs/plans/one-scene.md 1.4): each
//! shape table registered gives its entries the [`ShapeRef`]s the
//! scene's shape items name, in order after those registered before.
//! A closed form is drawn from its entry's geometry and the paint its
//! canonical bytes begin with; its parameters go in each instance's
//! bytes, so it uploads nothing (`shape_bytes_uploaded` is 0 for closed
//! forms). A mesh entry's triangles are uploaded once, at the start of
//! the first frame after it is registered (`shape_store_meshes.rs`).
//! The process registry (S5b) will register each crate's table once,
//! on its first mount. A document's run-time shapes (S4a) are taken
//! from its frame (`shape_store_runtime.rs`) into the same entries,
//! named by refs of the run-time range.

use std::sync::Arc;

use vapors_renderer::{ClosedFormShape, MeshEntry};
use vapors_scene::{Shape, ShapeKind, ShapeRef};

use crate::shape_bounds::outside_bounds;
pub use crate::shape_error::ShapeError;
use vapors_shapes::{Geometry, Paint, RuntimeShape, ShapeEntry, ShapeTable, paint_of};

/// The registered entries: a table entry at the index its
/// [`ShapeRef`] holds, a run-time one where `runtime` says.
#[derive(Debug)]
pub struct ShapeStore {
    entries: Vec<Registered>,
    /// Each run-time shape's entry index, by its index in its
    /// document's cache.
    pub(crate) runtime: Vec<usize>,
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
            runtime: Vec::new(),
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
    pub(crate) source: Source,
    /// The ref items name it by.
    pub(crate) shape: ShapeRef,
    /// The stroke width its paint names; `None` fills it.
    pub(crate) stroke: Option<f32>,
    /// A mesh's triangles on the GPU, once uploaded.
    pub(crate) uploaded: Option<MeshEntry>,
}

/// Where an entry came from: a crate's table, or a document's cache.
#[derive(Debug)]
pub(crate) enum Source {
    Table(&'static ShapeEntry),
    Runtime(Arc<RuntimeShape>),
}

impl Source {
    /// Where it was written.
    pub(crate) fn site(&self) -> &str {
        match self {
            Self::Table(entry) => entry.site,
            Self::Runtime(shape) => shape.site(),
        }
    }

    /// What it draws.
    pub(crate) fn geometry(&self) -> Geometry<'_> {
        match self {
            Self::Table(entry) => entry.geometry,
            Self::Runtime(shape) => shape.geometry(),
        }
    }
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
        let entries = table.entries().iter().zip(first..);
        self.entries
            .extend(entries.map(|(entry, index)| registered(entry, shape_ref(index))));
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
        let registered = self.of_kind(item, ShapeKind::ClosedForm)?;
        let (geometry, stroke) = (registered.source.geometry(), registered.stroke);
        let drawn = ClosedFormShape::new(&geometry, stroke, item.transform, item.colour)
            .map_err(|reason| undrawable(item, registered, reason))?;
        if drawn.is_some() {
            check_bounds(registered, stroke, item)?;
        }
        Ok(drawn)
    }

    /// The entry `item` names, when it is registered and of the kind
    /// both the item and the run it is drawn in say.
    pub(crate) fn of_kind(&self, item: &Shape, run: ShapeKind) -> Result<&Registered, ShapeError> {
        let registered = self.resolved(item.shape)?;
        let entry = kind_of(&registered.source.geometry());
        if item.kind() != entry || run != entry {
            return Err(ShapeError::Kind {
                shape: item.shape,
                site: registered.source.site().to_owned(),
                item: item.kind(),
                entry,
            });
        }
        Ok(registered)
    }

    /// The entry `shape` names, when the store holds one by that ref.
    fn resolved(&self, shape: ShapeRef) -> Result<&Registered, ShapeError> {
        let index = match shape.runtime_index() {
            Some(index) => self.runtime.get(index as usize).copied(),
            None => Some(shape.0 as usize),
        };
        index
            .and_then(|index| self.entries.get(index))
            .filter(|registered| registered.shape == shape)
            .ok_or(ShapeError::Unregistered {
                shape,
                registered: self.entries.len(),
            })
    }

    /// The entry index `shape` names; the store issued it.
    pub(crate) fn index_of(&self, shape: ShapeRef) -> usize {
        match shape.runtime_index() {
            Some(index) => self.runtime[index as usize],
            None => shape.0 as usize,
        }
    }

    /// The entry at `index`, which the store issued.
    pub(crate) fn at(&self, index: usize) -> &Registered {
        &self.entries[index]
    }

    pub(crate) fn entries_mut(&mut self) -> &mut Vec<Registered> {
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
    if geometry.is_closed_form() {
        ShapeKind::ClosedForm
    } else {
        ShapeKind::Mesh
    }
}

pub(crate) fn undrawable(
    item: &Shape,
    registered: &Registered,
    reason: vapors_renderer::ClosedFormError,
) -> ShapeError {
    ShapeError::Undrawable {
        shape: item.shape,
        site: registered.source.site().to_owned(),
        reason,
    }
}

/// Refuses `item` when what `registered` draws under it, stroked
/// `stroke` wide (a closed form's paint; a mesh holds its stroke),
/// lies outside its bounds, which culling reads.
pub(crate) fn check_bounds(
    registered: &Registered,
    stroke: Option<f32>,
    item: &Shape,
) -> Result<(), ShapeError> {
    let geometry = registered.source.geometry();
    match outside_bounds(&geometry, stroke, item) {
        None => Ok(()),
        Some(drawn) => Err(ShapeError::OutsideBounds {
            shape: item.shape,
            site: registered.source.site().to_owned(),
            drawn,
            bounds: item.bounds,
        }),
    }
}

fn shape_ref(index: usize) -> ShapeRef {
    ShapeRef(u32::try_from(index).expect("fewer than 2^32 shapes are registered"))
}

/// `entry`, checked by its table, and the stroke width its paint names.
fn registered(entry: &'static ShapeEntry, shape: ShapeRef) -> Registered {
    let paint = paint_of(entry.canonical).expect("the table's check read this paint");
    Registered {
        source: Source::Table(entry),
        shape,
        stroke: stroke_of(paint),
        uploaded: None,
    }
}

/// The stroke width `paint` names; `None` fills.
pub(crate) fn stroke_of(paint: Paint) -> Option<f32> {
    match paint {
        Paint::Fill(_) => None,
        Paint::Stroke { width } => Some(width),
    }
}

#[cfg(test)]
#[path = "shape_store_tests.rs"]
pub(crate) mod tests;
