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

use vapors_renderer::ClosedFormShape;
use vapors_scene::{Shape, ShapeKind, ShapeRef};

pub(crate) use crate::shape_entry::{Registered, Source, check_bounds, stroke_of, undrawable};
pub use crate::shape_entry::{kind_of, shape_item};
pub use crate::shape_error::ShapeError;
use vapors_shapes::{ShapeEntry, ShapeTable, paint_of};

/// The registered entries: a table entry at the index its
/// [`ShapeRef::Table`] holds, a run-time one in the slot its
/// [`ShapeRef::Runtime`] names, while the document's cache keeps it.
#[derive(Debug)]
pub struct ShapeStore {
    entries: Vec<Registered>,
    /// The run-time entries, by their slot in the document's cache.
    pub(crate) runtime: Vec<Option<Registered>>,
    /// The document whose run-time shapes the store holds.
    pub(crate) document: Option<u32>,
    /// Run-time slots whose meshes are not uploaded.
    pub(crate) runtime_unuploaded: Vec<u32>,
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
            document: None,
            runtime_unuploaded: Vec::new(),
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

impl ShapeStore {
    /// Registers `table`'s entries, after its check; each entry's ref,
    /// in the table's order.
    ///
    /// # Errors
    /// [`ShapeError::Table`] when the table's check refuses it (an
    /// undrawable geometry, a rect corner with one radius zero, bytes that
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
        Ok(self.closed_forms_in(shapes, &[])?.0)
    }

    /// As [`ShapeStore::closed_forms`], for items in a lane's rows: item
    /// `i` in row `rows[i]`, or none for an empty `rows`. The rows of the
    /// items drawn come back beside them.
    ///
    /// # Errors
    /// As [`ShapeStore::closed_forms`].
    pub(crate) fn closed_forms_in(
        &self,
        shapes: &[Shape],
        rows: &[u32],
    ) -> Result<(Vec<ClosedFormShape>, Vec<u32>), ShapeError> {
        drawn_in(shapes, rows, |shape| self.closed_form(shape))
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

    /// The entry `shape` names, when the store holds one by that ref (a
    /// run-time ref of its document, slot and generation).
    fn resolved(&self, shape: ShapeRef) -> Result<&Registered, ShapeError> {
        self.get(store_index(shape))
            .filter(|registered| registered.shape == shape)
            .ok_or(ShapeError::Unregistered {
                shape,
                registered: self.entries.len() + self.runtime.iter().flatten().count(),
            })
    }

    /// The entry at store index `index`, when one is there.
    fn get(&self, index: usize) -> Option<&Registered> {
        match index.checked_sub(RUNTIME_BASE) {
            Some(slot) => self.runtime.get(slot)?.as_ref(),
            None => self.entries.get(index),
        }
    }

    /// The entry at `index`, which the store issued.
    pub(crate) fn at(&self, index: usize) -> &Registered {
        self.get(index).expect("an index the store issued")
    }

    pub(crate) fn entries_mut(&mut self) -> &mut Vec<Registered> {
        &mut self.entries
    }
}

/// The store's index for `shape`: a table entry's index, or a run-time
/// slot past [`RUNTIME_BASE`].
pub(crate) fn store_index(shape: ShapeRef) -> usize {
    match shape {
        ShapeRef::Table(index) => index as usize,
        ShapeRef::Runtime(at) => RUNTIME_BASE + at.index as usize,
    }
}

/// Where run-time slots start in the store's index space: the top half
/// of `usize` (2^31 on wasm32), which `shape_ref` keeps table indices
/// below.
pub(crate) const RUNTIME_BASE: usize = usize::MAX / 2 + 1;

fn shape_ref(index: usize) -> ShapeRef {
    assert!(
        index < RUNTIME_BASE,
        "table entry {index} reaches the run-time slots' half of the store's index space"
    );
    ShapeRef::Table(u32::try_from(index).expect("fewer than 2^32 shapes are registered"))
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

/// What `draw` draws of `shapes`, and the rows of the items it draws:
/// item `i` is in row `rows[i]`, or `rows` is empty. An item that draws
/// nothing (no area) takes its row with it, so each drawn item keeps its
/// own.
///
/// # Errors
/// The first error `draw` returns.
pub(crate) fn drawn_in<T>(
    shapes: &[Shape],
    rows: &[u32],
    mut draw: impl FnMut(&Shape) -> Result<Option<T>, ShapeError>,
) -> Result<(Vec<T>, Vec<u32>), ShapeError> {
    let (mut drawn, mut kept) = (Vec::with_capacity(shapes.len()), Vec::new());
    for (at, shape) in shapes.iter().enumerate() {
        let Some(one) = draw(shape)? else { continue };
        drawn.push(one);
        kept.extend(rows.get(at));
    }
    Ok((drawn, kept))
}

#[cfg(test)]
#[path = "shape_store_tests.rs"]
pub(crate) mod tests;
