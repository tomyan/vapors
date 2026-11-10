//! Geometry on the GPU: each piece the application gave its `Meshes`,
//! uploaded once as a vertex and an index buffer.

use std::collections::HashMap;
use vapors_scene3d::{Geometry, GeometryId, Meshes, Vec3};
use wgpu::util::DeviceExt;

/// One piece's buffers.
pub(crate) struct Uploaded {
    pub(crate) vertices: wgpu::Buffer,
    pub(crate) indices: wgpu::Buffer,
    pub(crate) index_count: u32,
}

/// The pieces uploaded so far, by id.
#[derive(Default)]
pub(crate) struct GpuGeometry {
    uploaded: HashMap<GeometryId, Uploaded>,
}

impl GpuGeometry {
    /// Uploads the pieces queued on `meshes` since the last call.
    pub(crate) fn upload_queued(&mut self, device: &wgpu::Device, meshes: &mut Meshes) {
        for (id, geometry) in meshes.take_queued() {
            self.uploaded.insert(id, upload(device, &geometry));
        }
    }

    pub(crate) fn get(&self, id: GeometryId) -> Option<&Uploaded> {
        self.uploaded.get(&id)
    }
}

fn upload(device: &wgpu::Device, geometry: &Geometry) -> Uploaded {
    let positions: Vec<u8> = geometry
        .positions()
        .iter()
        .flat_map(Vec3::to_array)
        .flat_map(f32::to_le_bytes)
        .collect();
    let indices: Vec<u8> = geometry
        .indices()
        .iter()
        .flat_map(|i| i.to_le_bytes())
        .collect();
    Uploaded {
        vertices: buffer(
            device,
            "scene3d vertices",
            &positions,
            wgpu::BufferUsages::VERTEX,
        ),
        indices: buffer(
            device,
            "scene3d indices",
            &indices,
            wgpu::BufferUsages::INDEX,
        ),
        index_count: u32::try_from(geometry.indices().len())
            .expect("more than u32::MAX indices: wgpu cannot draw them in one call"),
    }
}

fn buffer(
    device: &wgpu::Device,
    label: &str,
    contents: &[u8],
    usage: wgpu::BufferUsages,
) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(label),
        contents,
        usage,
    })
}
