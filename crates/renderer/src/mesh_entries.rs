//! The GPU copies of a painter's mesh entries (docs/plans/one-scene.md
//! 1.4): each entry's triangles written once into a kept buffer and
//! drawn from there for as long as the painter lives; nothing is
//! released, as the shape store only grows. The bytes written are
//! counted, `shape_bytes_uploaded`, so a frame that registers nothing
//! writes none.

use vapors_shapes::MeshVertex;

use crate::{KeptRange, Retained, Target, WorkCounter};

/// Bytes per vertex: position, normal, three distances, three shifts.
pub(crate) const VERTEX_BYTES: u64 = 40;

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

/// `triangles`' vertices as the mesh pipeline reads them, padded with
/// zeros to a multiple of 16 bytes for the kept buffer.
fn vertex_bytes(triangles: &[[MeshVertex; 3]]) -> Vec<u8> {
    let mut bytes: Vec<u8> = triangles
        .iter()
        .flatten()
        .flat_map(|v| {
            let floats = [v.position.as_slice(), &v.normal, &v.edges, &v.shift].concat();
            floats
                .into_iter()
                .flat_map(f32::to_le_bytes)
                .collect::<Vec<u8>>()
        })
        .collect();
    debug_assert_eq!(
        bytes.len() as u64,
        triangles.len() as u64 * 3 * VERTEX_BYTES
    );
    bytes.resize(bytes.len().next_multiple_of(16), 0);
    bytes
}
