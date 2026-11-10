//! Geometry the application gives the scene: validated here, with no
//! GPU, and held for each scene renderer to upload.

use crate::MeshError;
use glam::Vec3;
use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

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
}

static NEXT_MESHES: AtomicU64 = AtomicU64::new(0);

/// The geometry an application has given, shared between it and the
/// scene renderers drawing it. Each piece is held from when it is given;
/// each renderer reads it with [`Meshes::get`] and uploads it once to
/// its own device, so several renderers can share one `Meshes`.
///
/// Nothing releases geometry yet, so every piece stays in memory while
/// the `Meshes` lives; release arrives in PR 5.
#[derive(Debug)]
pub struct Meshes {
    id: u64,
    issued: u64,
    held: HashMap<u64, Geometry>,
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

    /// Validates triangles and holds them for upload. A triangle whose
    /// indices repeat, or whose corners are collinear, is allowed and
    /// draws nothing.
    ///
    /// # Errors
    ///
    /// [`MeshError`]: no positions, fewer than 3, no indices, an index count
    /// that is not a multiple of 3, an index past the positions, a
    /// position that is NaN or infinite, or a position or index buffer
    /// past WebGPU's default `maxBufferSize` (256 MiB), the limit every
    /// adapter supports.
    pub fn geometry(
        &mut self,
        positions: Vec<Vec3>,
        indices: Vec<u32>,
    ) -> Result<GeometryId, MeshError> {
        crate::mesh_error::check(&positions, &indices)?;
        let id = GeometryId {
            meshes: self.id,
            index: self.issued,
        };
        self.issued += 1;
        self.held.insert(id.index, Geometry { positions, indices });
        Ok(id)
    }

    /// Whether this store issued `id`.
    #[must_use]
    pub fn issued(&self, id: GeometryId) -> bool {
        id.meshes == self.id && id.index < self.issued
    }

    /// The triangles under `id`; `None` when this store did not issue
    /// it, or no longer holds it.
    #[must_use]
    pub fn get(&self, id: GeometryId) -> Option<&Geometry> {
        if id.meshes != self.id {
            return None;
        }
        self.held.get(&id.index)
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
