//! The active camera's view-projection: one uniform every mesh draws
//! through, written only when it changed.

use crate::FrameError;
use crate::pipeline::{CAMERA_BYTES, Pipeline};
use vapors_app::SurfaceSize;
use vapors_scene3d::{Mat4, NodeKind, Scene};

pub(crate) struct CameraUniform {
    buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    written: Option<Mat4>,
}

impl CameraUniform {
    pub(crate) fn new(device: &wgpu::Device, pipeline: &Pipeline) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("scene3d camera"),
            size: CAMERA_BYTES,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("scene3d camera"),
            layout: &pipeline.camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });
        Self {
            buffer,
            bind_group,
            written: None,
        }
    }

    /// Writes `view_projection` unless it is the one last written.
    pub(crate) fn write(&mut self, queue: &wgpu::Queue, view_projection: Mat4) {
        if self.written == Some(view_projection) {
            return;
        }
        let bytes: Vec<u8> = view_projection
            .to_cols_array()
            .into_iter()
            .flat_map(f32::to_le_bytes)
            .collect();
        queue.write_buffer(&self.buffer, 0, &bytes);
        self.written = Some(view_projection);
    }

    pub(crate) fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}

/// World space to clip space through the active camera: its projection
/// for the backing's aspect, times the inverse of its world matrix.
/// `scene` must be updated.
pub(crate) fn view_projection(scene: &Scene, size: SurfaceSize) -> Result<Mat4, FrameError> {
    let camera = scene.active_camera()?;
    let NodeKind::Camera { projection, .. } = scene.kind(camera)? else {
        unreachable!("active_camera returns a camera node");
    };
    #[allow(clippy::cast_precision_loss)] // texture sizes are at most 2^14
    let aspect = size.width as f32 / size.height as f32;
    let view = scene.world(camera)?.inverse();
    let view_projection = projection.matrix(aspect)? * view;
    if view_projection.is_finite() {
        Ok(view_projection)
    } else {
        Err(FrameError::CameraNotFinite(camera))
    }
}
