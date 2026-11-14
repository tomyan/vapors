//! A document's run-time shapes in the store (one-scene S4a): before
//! each frame the painter hands the store its document's cache, and the
//! store takes the shapes past those it holds, each a [`ShapeRef`] of
//! the run-time range naming it by its index in the cache. A store
//! takes one document's: its cache only grows, so a shorter cache, or
//! one whose first shape is not the one the store took, is another
//! document's, which is refused.

use std::sync::Arc;

use vapors_scene::ShapeRef;
use vapors_shapes::{RuntimeShape, check_geometry, paint_of};

use crate::shape_store::{Registered, ShapeError, ShapeStore, Source, stroke_of};
use crate::shape_store_meshes::geometry_bytes;

impl ShapeStore {
    /// Takes the shapes of `cache` (a document's, by index) past those
    /// the store holds; their meshes upload at the frame's start.
    ///
    /// # Errors
    /// [`ShapeError::OtherDocument`] for a cache that is not the one the
    /// store took from; [`ShapeError::Runtime`] for a shape whose
    /// geometry or paint cannot be drawn; [`ShapeError::MeshBytes`] when
    /// their meshes would take the entry buffer past the device's
    /// limit. Nothing is taken then.
    pub fn take_runtime(&mut self, cache: &[Arc<RuntimeShape>]) -> Result<(), ShapeError> {
        self.check_same_document(cache)?;
        let taken = self.runtime.len();
        let new = &cache[taken..];
        let added = new
            .iter()
            .map(|shape| geometry_bytes(&shape.geometry()))
            .sum();
        let needed = self.mesh_bytes_with(added, || "the document's run-time shapes".to_owned())?;
        let made: Vec<Registered> = (taken..)
            .zip(new)
            .map(|(index, shape)| runtime_entry(index, shape))
            .collect::<Result<_, _>>()?;
        self.mesh_bytes = needed;
        for registered in made {
            let index = self.entries_mut().len();
            self.runtime.push(index);
            self.entries_mut().push(registered);
        }
        Ok(())
    }

    fn check_same_document(&self, cache: &[Arc<RuntimeShape>]) -> Result<(), ShapeError> {
        let first = self.runtime.first().map(|&index| &self.at(index).source);
        let same = match (first, cache.first()) {
            (None, _) => true,
            (Some(Source::Runtime(taken)), Some(given)) => Arc::ptr_eq(taken, given),
            _ => false,
        };
        if same && cache.len() >= self.runtime.len() {
            return Ok(());
        }
        Err(ShapeError::OtherDocument {
            held: self.runtime.len(),
            given: cache.len(),
        })
    }
}

/// The entry for the run-time shape at `index` in its document's cache.
fn runtime_entry(index: usize, shape: &Arc<RuntimeShape>) -> Result<Registered, ShapeError> {
    let refused = |reason: String| ShapeError::Runtime {
        site: shape.site().to_owned(),
        reason,
    };
    check_geometry(&shape.geometry()).map_err(refused)?;
    let paint = paint_of(shape.canonical().bytes()).map_err(refused)?;
    let index = u32::try_from(index).expect("a document caches fewer than 2^31 shapes");
    Ok(Registered {
        source: Source::Runtime(Arc::clone(shape)),
        shape: ShapeRef::runtime(index),
        stroke: stroke_of(paint),
        uploaded: None,
    })
}

#[cfg(test)]
#[path = "shape_store_runtime_tests.rs"]
mod tests;
