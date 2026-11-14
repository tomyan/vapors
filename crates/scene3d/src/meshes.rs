//! Geometry the application gives the scene: validated here, with no
//! GPU, and held for each scene renderer to upload.

use crate::MeshError;
use glam::Vec3;
use std::collections::HashMap;
use std::fmt;
use std::rc::Weak;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::GeometryHandle;

/// Geometry in one [`Meshes`]; a mesh node names it
/// (`NodeKind::Mesh`). It carries its `Meshes`' id, so a renderer can
/// refuse one from another `Meshes`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GeometryId {
    meshes: u64,
    index: u64,
}

impl fmt::Display for GeometryId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "geometry {} of meshes {}", self.index, self.meshes)
    }
}

/// Validated triangles: positions in the mesh's local space, and
/// indices into them, three per triangle.
#[derive(Debug, Clone, PartialEq)]
pub struct Geometry {
    positions: Vec<Vec3>,
    indices: Vec<u32>,
    shades: Vec<f32>,
}

impl Geometry {
    /// The positions, in the mesh node's local space.
    #[must_use]
    pub fn positions(&self) -> &[Vec3] {
        &self.positions
    }

    /// The indices, three per triangle, each within `positions`.
    #[must_use]
    pub fn indices(&self) -> &[u32] {
        &self.indices
    }

    /// Each position's shade, from 0 to 1: the fraction of the node's
    /// colour the vertex is drawn in (one-scene D4).
    #[must_use]
    pub fn shades(&self) -> &[f32] {
        &self.shades
    }
}

static NEXT_MESHES: AtomicU64 = AtomicU64::new(0);

/// The geometry an application has given, shared between it and the
/// scene renderers drawing it. Each piece is held from when it is given
/// while any [`GeometryHandle`] to it lives; each renderer reads it with
/// [`Meshes::get`] and uploads it once to its own device, so several
/// renderers can share one `Meshes`. A piece whose every handle is
/// dropped is released at the next [`Meshes::release_unclaimed`], which
/// each render of a `SceneRenderer` sharing this `Meshes` runs: until
/// one renders (a page idle and drawing on demand renders nothing), the
/// piece's data stays here. A render borrows the `Meshes` mutably for
/// that sweep, so holding a borrow of it across a frame panics there.
#[derive(Debug)]
pub struct Meshes {
    id: u64,
    issued: u64,
    held: HashMap<u64, Held>,
}

/// A held piece and the claim its handles share.
#[derive(Debug)]
struct Held {
    geometry: Geometry,
    claim: Weak<GeometryId>,
}

impl Meshes {
    /// An empty store with an id unique in this process.
    #[must_use]
    pub fn new() -> Self {
        Self {
            id: NEXT_MESHES.fetch_add(1, Ordering::Relaxed),
            issued: 0,
            held: HashMap::new(),
        }
    }

    /// Validates triangles, with a shade for each position, and holds
    /// them for upload. A triangle whose indices repeat, or whose corners
    /// are collinear, is allowed and draws nothing.
    ///
    /// # Errors
    ///
    /// [`MeshError`]: no positions, fewer than 3, no indices, an index count
    /// that is not a multiple of 3, an index past the positions, a
    /// position that is NaN or infinite, not one shade per position, a
    /// shade outside 0 to 1, or a vertex or index buffer past WebGPU's
    /// default `maxBufferSize` (256 MiB), the limit every adapter supports.
    pub fn geometry(
        &mut self,
        positions: Vec<Vec3>,
        indices: Vec<u32>,
        shades: Vec<f32>,
    ) -> Result<GeometryHandle, MeshError> {
        crate::mesh_error::check(&positions, &indices)?;
        crate::mesh_error::check_shades(&shades, positions.len())?;
        let id = GeometryId {
            meshes: self.id,
            index: self.issued,
        };
        self.issued += 1;
        let (handle, claim) = GeometryHandle::claim(id);
        let geometry = Geometry {
            positions,
            indices,
            shades,
        };
        self.held.insert(id.index, Held { geometry, claim });
        Ok(handle)
    }

    /// Whether this store issued `id`.
    #[must_use]
    pub fn issued(&self, id: GeometryId) -> bool {
        id.meshes == self.id && id.index < self.issued
    }

    /// The triangles under `id`; `None` when this store did not issue
    /// it, or has released it (a stale id).
    #[must_use]
    pub fn get(&self, id: GeometryId) -> Option<&Geometry> {
        if id.meshes != self.id {
            return None;
        }
        self.held.get(&id.index).map(|held| &held.geometry)
    }

    /// Whether this store holds `id`'s piece: issued here and not
    /// released.
    #[must_use]
    pub fn holds(&self, id: GeometryId) -> bool {
        self.get(id).is_some()
    }

    /// Releases every piece whose handles have all been dropped; their
    /// ids, from then on refused as stale by [`Meshes::get`].
    pub fn release_unclaimed(&mut self) -> Vec<GeometryId> {
        let meshes = self.id;
        let mut released = Vec::new();
        self.held.retain(|&index, held| {
            let claimed = held.claim.strong_count() > 0;
            if !claimed {
                released.push(GeometryId { meshes, index });
            }
            claimed
        });
        released.sort_by_key(|id| id.index);
        released
    }
}

impl Default for Meshes {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "meshes_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "meshes_release_tests.rs"]
mod release_tests;

#[cfg(test)]
#[path = "meshes_shades_tests.rs"]
mod shades_tests;
