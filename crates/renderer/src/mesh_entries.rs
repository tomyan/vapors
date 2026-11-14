//! The GPU copies of a painter's mesh entries (docs/plans/one-scene.md
//! 1.4): each entry's triangles written once into a kept buffer and
//! drawn from there until the store releases it (a run-time shape no
//! element paints any more). The bytes written are
//! counted, `shape_bytes_uploaded`, so a frame that registers nothing
//! writes none.

use std::mem::offset_of;

use vapors_shapes::{MeshCorner, MeshTriangle};

use crate::{KeptRange, Retained, Target, WorkCounter};

/// Bytes per triangle: `MeshTriangle`'s own size, the one answer the
/// buffer, the pipeline's stride and its attribute offsets read.
pub(crate) const TRIANGLE_BYTES: u64 = size_of::<MeshTriangle>() as u64;

/// The bytes `triangles` take in the entry buffer, padded to 16: the
/// one answer the store's registration limit, its upload count and the
/// buffer agree on.
#[must_use]
pub fn mesh_entry_bytes(triangles: usize) -> u64 {
    (triangles as u64 * TRIANGLE_BYTES).next_multiple_of(16)
}

/// One entry's triangles on the GPU.
#[derive(Debug)]
pub struct MeshEntry {
    pub(crate) range: KeptRange,
    pub(crate) triangles: u32,
}

/// The kept buffer of every mesh entry written.
pub struct MeshEntries {
    triangles: Retained,
    work: WorkCounter,
}

impl MeshEntries {
    /// An empty buffer.
    #[must_use]
    pub fn new(device: &wgpu::Device) -> Self {
        Self {
            triangles: Retained::new(device, "mesh entries", wgpu::BufferUsages::VERTEX),
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
    pub fn write(&mut self, target: &Target<'_>, triangles: &[MeshTriangle]) -> Option<MeshEntry> {
        assert!(
            !triangles.is_empty(),
            "an empty mesh reached the GPU; check_mesh refuses one"
        );
        let bytes = triangle_bytes(triangles);
        let range = self
            .triangles
            .keep(target.encoder, target.queue, &self.work, &bytes)?;
        let count = u32::try_from(triangles.len()).expect("fewer than 2^32 triangles in one mesh");
        Some(MeshEntry {
            range,
            triangles: count,
        })
    }

    /// Gives `entry`'s range back: its shape is no longer drawn (a
    /// run-time shape freed, one-scene S4a); its bytes are reused.
    pub fn release(&mut self, entry: MeshEntry) {
        self.triangles.release(entry.range);
    }

    /// The bytes the buffer holds.
    #[must_use]
    pub fn bytes(&self) -> u64 {
        self.triangles.live_bytes()
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
        self.triangles.slices(target.encoder, &ranges)
    }
}

/// `triangles` as the mesh pipeline reads them: each corner's fields
/// at their `offset_of!` in a triangle of `size_of::<MeshTriangle>()`,
/// the layout `meshes_setup.rs` declares from the same offsets; padded
/// with zeros to a multiple of 16 bytes for the kept buffer.
pub(crate) fn triangle_bytes(triangles: &[MeshTriangle]) -> Vec<u8> {
    let len =
        usize::try_from(mesh_entry_bytes(triangles.len())).expect("a mesh entry fits in memory");
    let mut bytes = vec![0; len];
    for (at, triangle) in triangles.iter().enumerate() {
        for (k, corner) in triangle.corners.iter().enumerate() {
            put_corner(&mut bytes, corner_offset(at, k), corner);
        }
    }
    bytes
}

/// Where corner `k` of triangle `at` starts.
const fn corner_offset(at: usize, k: usize) -> usize {
    at * size_of::<MeshTriangle>() + offset_of!(MeshTriangle, corners) + k * size_of::<MeshCorner>()
}

fn put_corner(bytes: &mut [u8], start: usize, corner: &MeshCorner) {
    put(
        bytes,
        start + offset_of!(MeshCorner, position),
        &corner.position,
    );
    put(
        bytes,
        start + offset_of!(MeshCorner, normals),
        corner.normals.as_flattened(),
    );
    put(bytes, start + offset_of!(MeshCorner, kind), &[corner.kind]);
    put(
        bytes,
        start + offset_of!(MeshCorner, reaches),
        &corner.reaches,
    );
}

fn put(bytes: &mut [u8], at: usize, floats: &[f32]) {
    for (k, float) in floats.iter().enumerate() {
        bytes[at + k * 4..at + k * 4 + 4].copy_from_slice(&float.to_le_bytes());
    }
}

#[cfg(test)]
#[path = "mesh_entries_tests.rs"]
mod tests;
