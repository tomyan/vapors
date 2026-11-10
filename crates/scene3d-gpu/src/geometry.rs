//! Geometry on the GPU: each piece a mesh node names, uploaded by this
//! renderer once as a vertex and an index buffer, at the first render
//! that draws it.

use crate::FrameError;
use crate::draw::MeshNode;
use std::collections::HashMap;
use vapors_scene3d::{Geometry, GeometryId, Vec3};
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
    /// Uploads each piece `nodes` name that this renderer has not,
    /// reading it through `find`.
    ///
    /// # Errors
    ///
    /// [`FrameError::GeometryMissing`] when `find` has no data for a
    /// piece not yet uploaded; nothing is uploaded then.
    pub(crate) fn upload_missing<'a>(
        &mut self,
        device: &wgpu::Device,
        nodes: &[MeshNode],
        find: impl Fn(GeometryId) -> Option<&'a Geometry>,
    ) -> Result<(), FrameError> {
        for (id, geometry) in self.missing(nodes, find)? {
            self.uploaded.insert(id, upload(device, geometry));
        }
        Ok(())
    }

    /// The pieces `nodes` name that are not uploaded, with their data.
    fn missing<'a>(
        &self,
        nodes: &[MeshNode],
        find: impl Fn(GeometryId) -> Option<&'a Geometry>,
    ) -> Result<HashMap<GeometryId, &'a Geometry>, FrameError> {
        let mut missing = HashMap::new();
        for mesh in nodes
            .iter()
            .filter(|mesh| !self.uploaded.contains_key(&mesh.geometry))
        {
            let geometry = find(mesh.geometry).ok_or(FrameError::GeometryMissing {
                node: mesh.node,
                geometry: mesh.geometry,
            })?;
            missing.insert(mesh.geometry, geometry);
        }
        Ok(missing)
    }

    /// `mesh`'s buffers.
    ///
    /// # Errors
    ///
    /// [`FrameError::GeometryMissing`] when its piece is not uploaded.
    pub(crate) fn get(&self, mesh: &MeshNode) -> Result<&Uploaded, FrameError> {
        self.uploaded
            .get(&mesh.geometry)
            .ok_or(FrameError::GeometryMissing {
                node: mesh.node,
                geometry: mesh.geometry,
            })
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

#[cfg(test)]
#[path = "geometry_tests.rs"]
mod tests;
