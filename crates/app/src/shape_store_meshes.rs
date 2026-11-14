//! The store's mesh entries (one-scene S3a2): each registered mesh's
//! vertices uploaded once, at the start of the first frame after it is
//! registered, and each mesh item's instance and entry for the mesh
//! pipeline.

use vapors_renderer::{GpuWork, MeshEntries, MeshEntry, MeshShape, Target, mesh_entry_bytes};
use vapors_scene::{Shape, ShapeKind, ShapeRef};
use vapors_shapes::{Geometry, ShapeTable};

use crate::shape_store::{Registered, ShapeError, ShapeStore, check_bounds, undrawable};

impl ShapeStore {
    /// The entry buffer's bytes with `table`'s meshes added.
    ///
    /// # Errors
    /// [`ShapeError::MeshBytes`] past the buffer's limit: the store
    /// only grows, so a table refused here can never be uploaded.
    pub(crate) fn check_mesh_bytes(&self, table: &ShapeTable) -> Result<u64, ShapeError> {
        let added: u64 = table
            .entries()
            .iter()
            .map(|entry| match entry.geometry {
                Geometry::Mesh { triangles } => mesh_entry_bytes(triangles.len()),
                _ => 0,
            })
            .sum();
        let needed = self.mesh_bytes.saturating_add(added);
        if needed > self.buffer_limit {
            return Err(ShapeError::MeshBytes {
                crate_name: table.crate_name(),
                needed,
                limit: self.buffer_limit,
            });
        }
        Ok(needed)
    }

    /// Uploads the vertices of every mesh entry registered since the
    /// last upload into `entries`; the work it issued, no bytes when
    /// nothing was registered. Registration keeps the buffer within the
    /// device's limit, so a write it refuses is a fault outside the
    /// store: the upload stops there (the refusal is on the target's
    /// encoder), each mesh after it is refused as not uploaded, and the
    /// next frame tries again from the same entry.
    pub(crate) fn upload(&mut self, entries: &mut MeshEntries, target: &Target<'_>) -> GpuWork {
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
        entries.take_work()
    }

    /// `shapes`' instances for the mesh pipeline, and beside each the
    /// store's index of its entry: the one answer to which items draw
    /// (an item whose transform has no area is left out of both).
    ///
    /// # Errors
    /// As [`ShapeStore::closed_forms`], for a mesh run: an item whose
    /// entry is a closed form is [`ShapeError::Kind`].
    pub(crate) fn mesh_instances(
        &self,
        shapes: &[Shape],
    ) -> Result<(Vec<MeshShape>, Vec<usize>), ShapeError> {
        let (mut instances, mut entries) = (Vec::new(), Vec::new());
        for item in shapes {
            let registered = self.of_kind(item, ShapeKind::Mesh)?;
            let mesh = MeshShape::new(item.transform, item.colour)
                .map_err(|reason| undrawable(item, registered.entry, reason))?;
            if let Some(mesh) = mesh {
                check_bounds(registered.entry, None, item)?;
                instances.push(mesh);
                entries.push(item.shape.0 as usize);
            }
        }
        Ok((instances, entries))
    }

    /// The uploaded entry at each of `indices` (from
    /// [`ShapeStore::mesh_instances`]), for a kept run's draw.
    ///
    /// # Errors
    /// [`ShapeError::NotUploaded`] for an entry whose vertices are not
    /// on the GPU.
    pub(crate) fn uploaded(&self, indices: &[usize]) -> Result<Vec<&MeshEntry>, ShapeError> {
        indices
            .iter()
            .map(|&index| uploaded(index, self.at(index)))
            .collect()
    }
}

fn uploaded(index: usize, registered: &Registered) -> Result<&MeshEntry, ShapeError> {
    registered.uploaded.as_ref().ok_or(ShapeError::NotUploaded {
        shape: ShapeRef(u32::try_from(index).expect("an index the store issued")),
        site: registered.entry.site,
    })
}

#[cfg(test)]
#[path = "shape_store_meshes_tests.rs"]
mod tests;
