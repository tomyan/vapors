//! The GPU copies of a painter's mesh entries (docs/plans/one-scene.md
//! 1.4): each entry's triangles written once into a kept buffer and
//! drawn from there for as long as the painter lives; nothing is
//! released, as the shape store only grows. The bytes written are
//! counted, `shape_bytes_uploaded`, so a frame that registers nothing
//! writes none.

use std::mem::offset_of;

use vapors_shapes::MeshVertex;

use crate::{KeptRange, Retained, Target, WorkCounter};

/// Bytes per vertex: `MeshVertex`'s own size, the one answer the
/// buffer, the pipeline's stride and its attribute offsets read.
pub(crate) const VERTEX_BYTES: u64 = size_of::<MeshVertex>() as u64;

/// One entry's triangles on the GPU.
#[derive(Debug)]
pub struct MeshEntry {
    pub(crate) range: KeptRange,
    pub(crate) vertices: u32,
}

/// The kept buffer of every mesh entry written.
pub struct MeshEntries {
    vertices: Retained,
    work: WorkCounter,
}

impl MeshEntries {
    /// An empty buffer.
    #[must_use]
    pub fn new(device: &wgpu::Device) -> Self {
        Self {
            vertices: Retained::new(device, "mesh entries", wgpu::BufferUsages::VERTEX),
            work: WorkCounter::default(),
        }
    }

    /// Writes `triangles` for draws recorded into `target`'s encoder and
    /// later ones; `None` when the buffer they need is over the device's
    /// `max_buffer_size` (the refusal is on the encoder).
    ///
    /// # Panics
    /// When `triangles` is empty: the table's check refuses an empty
    /// mesh.
    pub fn write(
        &mut self,
        target: &Target<'_>,
        triangles: &[[MeshVertex; 3]],
    ) -> Option<MeshEntry> {
        assert!(
            !triangles.is_empty(),
            "an empty mesh reached the GPU; check_mesh refuses one"
        );
        let bytes = vertex_bytes(triangles);
        let range = self
            .vertices
            .keep(target.encoder, target.queue, &self.work, &bytes)?;
        let vertices =
            u32::try_from(triangles.len() * 3).expect("fewer than 2^32 vertices in one mesh");
        Some(MeshEntry { range, vertices })
    }

    /// The bytes the buffer holds.
    #[must_use]
    pub fn bytes(&self) -> u64 {
        self.vertices.live_bytes()
    }

    /// What its writes issued since the last take; the count restarts.
    pub fn take_work(&self) -> crate::GpuWork {
        self.work.take()
    }

    pub(crate) fn slices<'a>(
        &'a mut self,
        target: &Target<'_>,
        entries: &[&MeshEntry],
    ) -> Vec<wgpu::BufferSlice<'a>> {
        let ranges: Vec<&KeptRange> = entries.iter().map(|entry| &entry.range).collect();
        self.vertices.slices(target.encoder, &ranges)
    }
}

/// `triangles`' vertices as the mesh pipeline reads them: each field's
/// floats at its `offset_of!` in a vertex of `size_of::<MeshVertex>()`,
/// the layout `meshes_setup.rs` declares from the same offsets; padded
/// with zeros to a multiple of 16 bytes for the kept buffer.
pub(crate) fn vertex_bytes(triangles: &[[MeshVertex; 3]]) -> Vec<u8> {
    let size = size_of::<MeshVertex>();
    let mut bytes = vec![0; (triangles.len() * 3 * size).next_multiple_of(16)];
    for (at, vertex) in triangles.iter().flatten().enumerate() {
        let start = at * size;
        put(
            &mut bytes,
            start + offset_of!(MeshVertex, position),
            &vertex.position,
        );
        put(
            &mut bytes,
            start + offset_of!(MeshVertex, normal),
            &vertex.normal,
        );
        put(
            &mut bytes,
            start + offset_of!(MeshVertex, edges),
            &vertex.edges,
        );
        put(
            &mut bytes,
            start + offset_of!(MeshVertex, edge_normals),
            vertex.edge_normals.as_flattened(),
        );
    }
    bytes
}

fn put(bytes: &mut [u8], at: usize, floats: &[f32]) {
    for (k, float) in floats.iter().enumerate() {
        bytes[at + k * 4..at + k * 4 + 4].copy_from_slice(&float.to_le_bytes());
    }
}

#[cfg(test)]
#[path = "mesh_entries_tests.rs"]
mod tests;
