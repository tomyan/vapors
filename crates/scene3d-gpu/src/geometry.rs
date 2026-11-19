//! Geometry on the GPU: each piece a mesh node names, uploaded by this
//! renderer once as a vertex and an index buffer, at the first render
//! that draws it, through the frame's work counter.

use crate::FrameError;
use crate::draw::MeshNode;
use crate::log::Tracked;
use crate::{RendererLog, Resource};
use std::collections::HashMap;
use std::rc::Rc;
use vapors_app::SurfaceFrame;
use vapors_scene3d::{Geometry, GeometryId, Vec3};

/// One piece's buffers.
pub(crate) struct Uploaded {
    pub(crate) vertices: wgpu::Buffer,
    pub(crate) indices: wgpu::Buffer,
    /// The normals, in a buffer of their own (vertex buffer slot 1,
    /// [`vapors_scene3d::NORMAL_BYTES`] a vertex), only when the geometry
    /// has them. The unlit pipeline binds only `vertices`, so it ignores
    /// them; the lit pipeline (LT3) binds both.
    #[allow(dead_code)] // read by the lit pipeline (LT3)
    pub(crate) normals: Option<wgpu::Buffer>,
    pub(crate) index_count: u32,
}

/// The pieces uploaded so far, by id, each recorded in the log.
pub(crate) struct GpuGeometry {
    uploaded: HashMap<GeometryId, Tracked<Uploaded>>,
    log: Rc<RendererLog>,
}

impl GpuGeometry {
    pub(crate) fn new(log: &Rc<RendererLog>) -> Self {
        Self {
            uploaded: HashMap::new(),
            log: Rc::clone(log),
        }
    }

    /// Uploads each piece `nodes` name that this renderer has not,
    /// reading it through each node's handle.
    pub(crate) fn upload_missing(&mut self, frame: &SurfaceFrame<'_>, nodes: &[MeshNode]) {
        for (id, geometry) in self.missing(nodes) {
            let uploaded = Tracked::new(upload(frame, geometry), &self.log, Resource::Geometry);
            self.uploaded.insert(id, uploaded);
        }
    }

    /// The pieces `nodes` name that are not uploaded, with their data.
    fn missing<'a>(&self, nodes: &'a [MeshNode]) -> HashMap<GeometryId, &'a Geometry> {
        (nodes.iter())
            .filter(|mesh| !self.uploaded.contains_key(&mesh.geometry))
            .map(|mesh| (mesh.geometry, mesh.handle.geometry()))
            .collect()
    }

    /// Drops the buffers of each piece `held` says its `Meshes` released,
    /// which the log counts freed.
    pub(crate) fn forget_released(&mut self, held: impl Fn(GeometryId) -> bool) {
        self.uploaded.retain(|&id, _| held(id));
    }

    /// `mesh`'s buffers.
    ///
    /// # Errors
    ///
    /// [`FrameError::GeometryMissing`] when its piece is not uploaded.
    pub(crate) fn get(&self, mesh: &MeshNode) -> Result<&Uploaded, FrameError> {
        self.uploaded
            .get(&mesh.geometry)
            .map(|uploaded| &**uploaded)
            .ok_or(FrameError::GeometryMissing {
                node: mesh.node,
                geometry: mesh.geometry,
            })
    }
}

/// Each vertex's position's three floats, then its shade
/// ([`vapors_scene3d::VERTEX_BYTES`] a vertex).
fn vertex_bytes(geometry: &Geometry) -> Vec<u8> {
    (geometry.positions().iter().zip(geometry.shades()))
        .flat_map(|(position, shade)| position.extend(*shade).to_array())
        .flat_map(f32::to_le_bytes)
        .collect()
}

/// Each vertex's normal's three floats, when the geometry has normals.
fn normal_bytes(geometry: &Geometry) -> Option<Vec<u8>> {
    let normals = geometry.normals()?;
    Some(
        normals
            .iter()
            .flat_map(Vec3::to_array)
            .flat_map(f32::to_le_bytes)
            .collect(),
    )
}

fn upload(frame: &SurfaceFrame<'_>, geometry: &Geometry) -> Uploaded {
    let positions = vertex_bytes(geometry);
    let indices: Vec<u8> = geometry
        .indices()
        .iter()
        .flat_map(|i| i.to_le_bytes())
        .collect();
    Uploaded {
        vertices: buffer(
            frame,
            "scene3d vertices",
            &positions,
            wgpu::BufferUsages::VERTEX,
        ),
        indices: buffer(
            frame,
            "scene3d indices",
            &indices,
            wgpu::BufferUsages::INDEX,
        ),
        normals: normal_bytes(geometry).map(|bytes| {
            let usage = wgpu::BufferUsages::VERTEX;
            buffer(frame, "scene3d normals", &bytes, usage)
        }),
        index_count: u32::try_from(geometry.indices().len())
            .expect("more than u32::MAX indices: wgpu cannot draw them in one call"),
    }
}

/// A buffer holding `contents`, written through `frame.work`.
/// `contents` is 4-byte floats or indices, so its length is a multiple
/// of `wgpu::COPY_BUFFER_ALIGNMENT`.
fn buffer(
    frame: &SurfaceFrame<'_>,
    label: &str,
    contents: &[u8],
    usage: wgpu::BufferUsages,
) -> wgpu::Buffer {
    let buffer = frame.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: contents.len() as u64,
        usage: usage | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    frame.work.write_buffer(frame.queue, &buffer, 0, contents);
    buffer
}

#[cfg(test)]
#[path = "geometry_tests.rs"]
mod tests;
