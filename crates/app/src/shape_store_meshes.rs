//! The store's mesh entries (one-scene S3a2): each registered mesh's
//! triangles uploaded once, at the start of the first frame after it is
//! registered, and each mesh item's instance and entry for the mesh
//! pipeline.

use vapors_renderer::{GpuWork, MeshEntries, MeshEntry, MeshShape, Target, mesh_entry_bytes};
use vapors_scene::{Shape, ShapeKind};
use vapors_shapes::{Geometry, ShapeTable};

use crate::shape_store::{Registered, ShapeError, ShapeStore, check_bounds, undrawable};

/// A mesh run's drawn items ([`ShapeStore::mesh_instances`]).
pub(crate) struct MeshRun {
    pub(crate) instances: Vec<MeshShape>,
    /// The store's index of each instance's entry.
    pub(crate) entries: Vec<usize>,
    /// Each instance's row, or none for a run in one frame.
    pub(crate) rows: Vec<u32>,
}

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
            .map(|entry| geometry_bytes(&entry.geometry))
            .sum();
        self.mesh_bytes_with(added, || {
            format!("shape table of crate {}", table.crate_name())
        })
    }

    /// The entry buffer's bytes with `added` more.
    ///
    /// # Errors
    /// [`ShapeError::MeshBytes`] past the buffer's limit, naming what
    /// `owner` says would add them.
    pub(crate) fn mesh_bytes_with(
        &self,
        added: u64,
        owner: impl FnOnce() -> String,
    ) -> Result<u64, ShapeError> {
        let needed = self.mesh_bytes.saturating_add(added);
        if needed > self.buffer_limit {
            return Err(ShapeError::MeshBytes {
                owner: owner(),
                needed,
                limit: self.buffer_limit,
            });
        }
        Ok(needed)
    }

    /// Uploads the triangles of every mesh entry registered since the
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
            if !upload_one(registered, entries, target) {
                break;
            }
            done += 1;
        }
        self.unuploaded += done;
        self.upload_runtime(entries, target);
        entries.take_work()
    }

    /// Uploads each run-time slot taken since the last upload, stopping
    /// at a refused write as [`ShapeStore::upload`] does.
    fn upload_runtime(&mut self, entries: &mut MeshEntries, target: &Target<'_>) {
        let pending = std::mem::take(&mut self.runtime_unuploaded);
        for (at, &index) in pending.iter().enumerate() {
            let Some(registered) = self.runtime[index as usize].as_mut() else {
                continue;
            };
            if registered.uploaded.is_none() && !upload_one(registered, entries, target) {
                self.runtime_unuploaded = pending[at..].to_vec();
                return;
            }
        }
    }

    /// `shapes`' instances for the mesh pipeline, beside each the
    /// store's index of its entry, and, for items in a lane's rows (item
    /// `i` in row `rows[i]`, or `rows` empty), the rows of the items
    /// drawn: the one answer to which items draw (an item whose transform
    /// has no area is left out of all three).
    ///
    /// # Errors
    /// As [`ShapeStore::closed_forms`], for a mesh run: an item whose
    /// entry is a closed form is [`ShapeError::Kind`].
    pub(crate) fn mesh_instances(
        &self,
        shapes: &[Shape],
        rows: &[u32],
    ) -> Result<MeshRun, ShapeError> {
        let (drawn, rows) =
            crate::shape_store::drawn_in(shapes, rows, |item| self.mesh_instance(item))?;
        let (instances, entries) = drawn.into_iter().unzip();
        Ok(MeshRun {
            instances,
            entries,
            rows,
        })
    }

    /// `item`'s instance and its entry's index; `None` when its
    /// transform has no area.
    fn mesh_instance(&self, item: &Shape) -> Result<Option<(MeshShape, usize)>, ShapeError> {
        let registered = self.of_kind(item, ShapeKind::Mesh)?;
        let mesh = MeshShape::new(item.transform, item.colour)
            .map_err(|reason| undrawable(item, registered, reason))?;
        let Some(mesh) = mesh else { return Ok(None) };
        check_bounds(registered, None, item)?;
        Ok(Some((mesh, crate::shape_store::store_index(item.shape))))
    }

    /// The uploaded entry at each of `indices` (from
    /// [`ShapeStore::mesh_instances`]), for a kept run's draw.
    ///
    /// # Errors
    /// [`ShapeError::NotUploaded`] for an entry whose triangles are not
    /// on the GPU.
    pub(crate) fn uploaded(&self, indices: &[usize]) -> Result<Vec<&MeshEntry>, ShapeError> {
        indices
            .iter()
            .map(|&index| uploaded(self.at(index)))
            .collect()
    }
}

fn uploaded(registered: &Registered) -> Result<&MeshEntry, ShapeError> {
    registered
        .uploaded
        .as_ref()
        .ok_or_else(|| ShapeError::NotUploaded {
            shape: registered.shape,
            site: registered.source.site().to_owned(),
        })
}

/// Writes `registered`'s triangles, when it is a mesh; whether the
/// buffer took them (a closed form needs none).
fn upload_one(registered: &mut Registered, entries: &mut MeshEntries, target: &Target<'_>) -> bool {
    let Geometry::Mesh { triangles } = registered.source.geometry() else {
        return true;
    };
    let Some(written) = entries.write(target, triangles) else {
        return false;
    };
    registered.uploaded = Some(written);
    true
}

/// The entry buffer's bytes `geometry` takes: a mesh's, 0 for a
/// closed form.
pub(crate) fn geometry_bytes(geometry: &Geometry<'_>) -> u64 {
    match geometry {
        Geometry::Mesh { triangles } => mesh_entry_bytes(triangles.len()),
        _ => 0,
    }
}

#[cfg(test)]
#[path = "shape_store_meshes_tests.rs"]
mod tests;
