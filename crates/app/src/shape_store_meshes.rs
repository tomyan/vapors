//! The store's mesh entries (one-scene S3a2): each registered mesh's
//! vertices uploaded once, at the start of the first frame after it is
//! registered, and each mesh item's instance and entry for the mesh
//! pipeline.

use vapors_renderer::{MeshEntries, MeshEntry, MeshShape, Target};
use vapors_scene::{Shape, ShapeKind};
use vapors_shapes::Geometry;

use crate::shape_store::{Registered, ShapeError, ShapeStore, check_bounds, undrawable};

impl ShapeStore {
    /// Uploads the vertices of every mesh entry registered since the
    /// last upload into `entries`; the bytes written, 0 when nothing
    /// was registered. A write the buffer refuses stops the upload (the
    /// refusal is on the target's encoder) and is tried again next frame.
    pub(crate) fn upload(&mut self, entries: &mut MeshEntries, target: &Target<'_>) -> u64 {
        let first = self.unuploaded;
        let mut done = 0;
        for registered in &mut self.entries_mut()[first..] {
            if let Geometry::Mesh { triangles } = registered.entry.geometry {
                let Some(written) = entries.write(target, triangles) else {
                    break;
                };
                registered.uploaded = Some(written);
            }
            done += 1;
        }
        self.unuploaded += done;
        entries.take_work().buffer_bytes
    }

    /// `shapes`' instances for the mesh pipeline, items whose transform
    /// has no area left out.
    ///
    /// # Errors
    /// As [`ShapeStore::closed_forms`], for a mesh run: an item whose
    /// entry is a closed form is [`ShapeError::Kind`].
    pub(crate) fn mesh_instances(&self, shapes: &[Shape]) -> Result<Vec<MeshShape>, ShapeError> {
        let mut drawn = Vec::with_capacity(shapes.len());
        for item in shapes {
            let registered = self.of_kind(item, ShapeKind::Mesh)?;
            let mesh = MeshShape::new(item.transform, item.colour)
                .map_err(|reason| undrawable(item, registered.entry, reason))?;
            if mesh.is_some() {
                check_bounds(registered.entry, None, item)?;
            }
            drawn.extend(mesh);
        }
        Ok(drawn)
    }

    /// The uploaded entry of each of `shapes` that draws, beside its
    /// instance in [`ShapeStore::mesh_instances`].
    ///
    /// # Errors
    /// As [`ShapeStore::mesh_instances`]; [`ShapeError::NotUploaded`]
    /// for an entry whose vertices are not on the GPU.
    pub(crate) fn mesh_entries(&self, shapes: &[Shape]) -> Result<Vec<&MeshEntry>, ShapeError> {
        let mut drawn = Vec::with_capacity(shapes.len());
        for item in shapes {
            let registered = self.of_kind(item, ShapeKind::Mesh)?;
            let has_area = MeshShape::new(item.transform, item.colour)
                .map_err(|reason| undrawable(item, registered.entry, reason))?
                .is_some();
            if has_area {
                drawn.push(uploaded(item, registered)?);
            }
        }
        Ok(drawn)
    }
}

fn uploaded<'a>(item: &Shape, registered: &'a Registered) -> Result<&'a MeshEntry, ShapeError> {
    registered.uploaded.as_ref().ok_or(ShapeError::NotUploaded {
        shape: item.shape,
        site: registered.entry.site,
    })
}

#[cfg(test)]
#[path = "shape_store_meshes_tests.rs"]
mod tests;
