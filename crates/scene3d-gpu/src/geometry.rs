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
use vapors_scene3d::{Geometry, GeometryId};

/// One piece's buffers.
pub(crate) struct Uploaded {
    pub(crate) vertices: wgpu::Buffer,
    pub(crate) indices: wgpu::Buffer,
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
    /// reading it through `find`.
    ///
    /// # Errors
    ///
    /// [`FrameError::GeometryMissing`] when `find` has no data for a
    /// piece not yet uploaded; nothing is uploaded then.
    pub(crate) fn upload_missing<'a>(
        &mut self,
        frame: &SurfaceFrame<'_>,
        nodes: &[MeshNode],
        find: impl Fn(GeometryId) -> Option<&'a Geometry>,
    ) -> Result<(), FrameError> {
        for (id, geometry) in self.missing(nodes, find)? {
            let uploaded = Tracked::new(upload(frame, geometry), &self.log, Resource::Geometry);
            self.uploaded.insert(id, uploaded);
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

fn upload(frame: &SurfaceFrame<'_>, geometry: &Geometry) -> Uploaded {
    // each vertex: its position's three floats, then its shade
    let positions: Vec<u8> = (geometry.positions().iter().zip(geometry.shades()))
        .flat_map(|(position, shade)| position.extend(*shade).to_array())
        .flat_map(f32::to_le_bytes)
        .collect();
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
