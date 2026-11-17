//! The mesh pipeline's item bytes (`meshes.rs`): an item's floats
//! ([`MeshShape::instance`]), then its row's index in its batch's range
//! and its frame's entry in the transform table, padded to 16 bytes as
//! kept bytes are (`retained.rs`). Each draw reads one item's bytes at
//! stride 0 (`meshes_setup.rs`).

use crate::instanced::float_bytes;
use crate::{MeshPipeline, MeshShape};

/// Bytes per item, for slicing the kept buffer.
pub(crate) const INSTANCE_BYTES: u64 = MeshPipeline::INSTANCE_BYTES as u64;

impl MeshPipeline {
    /// Bytes per item in the kept buffer: [`MeshShape::instance`]'s
    /// floats, then its row's index and its frame's entry, padded to 16.
    pub const INSTANCE_BYTES: usize = crate::mesh_shape::INSTANCE_FLOATS * 4 + 16;

    /// The item bytes of `shapes`, as a draw keeps them: each on the
    /// page.
    #[must_use]
    pub fn bytes(shapes: &[MeshShape]) -> Vec<u8> {
        Self::bytes_in(shapes, 0, &[])
    }

    /// The item bytes of `shapes`, each in frame `frame` (its entry in
    /// the transform table), item `i` in row `rows[i]` of its batch's
    /// range, or every item in row 0 for an empty `rows`.
    ///
    /// # Panics
    ///
    /// As [`QuadPipeline::bytes_in`](crate::QuadPipeline::bytes_in).
    #[must_use]
    pub fn bytes_in(shapes: &[MeshShape], frame: u32, rows: &[u32]) -> Vec<u8> {
        crate::instanced_bytes::assert_rows("meshes", shapes.len(), rows);
        let mut bytes = Vec::with_capacity(shapes.len() * Self::INSTANCE_BYTES);
        for (at, shape) in shapes.iter().enumerate() {
            bytes.extend(float_bytes(shape.instance()));
            bytes.extend(rows.get(at).copied().unwrap_or(0).to_le_bytes());
            bytes.extend(frame.to_le_bytes());
            bytes.extend([0; 8]);
        }
        bytes
    }
}

#[cfg(test)]
#[path = "meshes_bytes_tests.rs"]
mod tests;
