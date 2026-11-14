//! A document's run-time shapes in the store (one-scene S4a): before
//! each frame the painter hands the store the slots of its document's
//! cache that changed, and the store takes each as it is now: a new
//! entry in the slot, its meshes uploaded at the frame's start, or the
//! slot freed, its GPU range given back. An item names a run-time entry
//! by its document, slot and generation, so a ref to an entry since
//! freed, or another document's, is refused. A store takes one
//! document's changes.

use std::sync::Arc;

use vapors_renderer::MeshEntry;
use vapors_scene::{RuntimeRef, ShapeRef};
use vapors_shapes::{RuntimeShape, SlotChange, check_geometry, paint_of};

use crate::shape_store::{Registered, ShapeError, ShapeStore, Source, stroke_of};
use crate::shape_store_meshes::geometry_bytes;

impl ShapeStore {
    /// Takes `changes` (slots of `document`'s cache, each with its entry
    /// now); the GPU entries of the meshes they free or replace, for the
    /// caller to give back to its entry buffer.
    ///
    /// # Errors
    /// [`ShapeError::OtherDocument`] for another document's changes;
    /// [`ShapeError::Runtime`] for an entry whose geometry or paint
    /// cannot be drawn; [`ShapeError::MeshBytes`] when the new meshes
    /// would take the entry buffer past the device's limit. Nothing is
    /// taken then.
    pub fn take_runtime(
        &mut self,
        document: u32,
        changes: Vec<SlotChange>,
    ) -> Result<Vec<MeshEntry>, ShapeError> {
        self.check_document(document)?;
        let made: Vec<(u32, Option<Registered>)> = changes
            .into_iter()
            .map(|(index, now)| {
                Ok((
                    index,
                    now.map(|shape| runtime_entry(document, index, shape))
                        .transpose()?,
                ))
            })
            .collect::<Result<_, ShapeError>>()?;
        self.mesh_bytes = self.mesh_bytes_with(added_bytes(&made), || {
            "the document's run-time shapes".to_owned()
        })? - self.freed_bytes(&made);
        self.document = Some(document);
        Ok(made
            .into_iter()
            .filter_map(|(index, now)| self.put(index, now))
            .collect())
    }

    /// Takes the changed run-time shapes of `painted`'s document, and
    /// gives the GPU ranges of those freed back to `meshes`.
    ///
    /// # Errors
    /// As [`ShapeStore::take_runtime`].
    pub(crate) fn take_from(
        &mut self,
        painted: &vapors_elements::Painted,
        meshes: &mut vapors_renderer::MeshEntries,
    ) -> Result<(), ShapeError> {
        let (document, changes) = painted.shape_changes();
        for freed in self.take_runtime(document, changes)? {
            meshes.release(freed);
        }
        // only once taken: a refused take keeps them for the next frame
        painted.shape_changes_taken();
        Ok(())
    }

    fn check_document(&self, document: u32) -> Result<(), ShapeError> {
        match self.document {
            Some(held) if held != document => Err(ShapeError::OtherDocument {
                held,
                given: document,
            }),
            _ => Ok(()),
        }
    }

    /// The bytes of the meshes the changes replace or free.
    fn freed_bytes(&self, made: &[(u32, Option<Registered>)]) -> u64 {
        made.iter()
            .filter_map(|(index, _)| self.runtime.get(*index as usize)?.as_ref())
            .map(|old| geometry_bytes(&old.source.geometry()))
            .sum()
    }

    /// Slot `index` holds `now`; the GPU entry of what it held.
    fn put(&mut self, index: u32, now: Option<Registered>) -> Option<MeshEntry> {
        let slot = index as usize;
        if self.runtime.len() <= slot {
            self.runtime.resize_with(slot + 1, || None);
        }
        if now.is_some() {
            self.runtime_unuploaded.push(index);
        }
        let old = std::mem::replace(&mut self.runtime[slot], now);
        old.and_then(|old| old.uploaded)
    }
}

/// The bytes of the meshes the changes add.
fn added_bytes(made: &[(u32, Option<Registered>)]) -> u64 {
    let added = made.iter().filter_map(|(_, now)| now.as_ref());
    added
        .map(|registered| geometry_bytes(&registered.source.geometry()))
        .sum()
}

/// The entry for `shape` in slot `index` of `document`'s cache.
fn runtime_entry(
    document: u32,
    index: u32,
    (generation, shape): (u32, Arc<RuntimeShape>),
) -> Result<Registered, ShapeError> {
    let refused = |reason: String| ShapeError::Runtime {
        site: shape.site().to_owned(),
        reason,
    };
    check_geometry(&shape.geometry()).map_err(refused)?;
    let paint = paint_of(shape.canonical().bytes()).map_err(refused)?;
    let at = RuntimeRef {
        document,
        index,
        generation,
    };
    Ok(Registered {
        source: Source::Runtime(shape),
        shape: ShapeRef::Runtime(at),
        stroke: stroke_of(paint),
        uploaded: None,
    })
}

#[cfg(test)]
#[path = "shape_store_runtime_tests.rs"]
mod tests;
