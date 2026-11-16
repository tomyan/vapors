//! The active camera's view-projection and the backing's size in device
//! pixels (`vapors_renderer::projective`'s camera uniform): one uniform
//! every mesh draws through, written only when it changed.

use crate::FrameError;
use crate::pipeline::{CAMERA_BYTES, Pipeline};
use vapors_app::{SurfaceFrame, SurfaceSize};
use vapors_scene3d::{Mat4, Scene};

pub(crate) struct CameraUniform {
    buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    written: Option<(Mat4, SurfaceSize)>,
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
            layout: &pipeline.layouts.camera,
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

    /// Writes `view_projection` and the backing's size through
    /// `frame.work` unless they are the ones last written.
    pub(crate) fn write(&mut self, frame: &SurfaceFrame<'_>, view_projection: Mat4) {
        let now = (view_projection, frame.size);
        if self.written == Some(now) {
            return;
        }
        #[allow(clippy::cast_precision_loss)] // texture sizes are at most 2^14
        let size = [frame.size.width as f32, frame.size.height as f32, 0.0, 0.0];
        let bytes: Vec<u8> = view_projection
            .to_cols_array()
            .into_iter()
            .chain(size)
            .flat_map(f32::to_le_bytes)
            .collect();
        frame
            .work
            .write_buffer(frame.queue, &self.buffer, 0, &bytes);
        self.written = Some(now);
    }

    pub(crate) fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}

/// World space to clip space through the active camera at the
/// backing's aspect ([`Scene::view_projection`], which `pick` uses too).
/// `scene` must be updated.
pub(crate) fn view_projection(scene: &Scene, size: SurfaceSize) -> Result<Mat4, FrameError> {
    #[allow(clippy::cast_precision_loss)] // texture sizes are at most 2^14
    let aspect = size.width as f32 / size.height as f32;
    Ok(scene.view_projection(aspect)?)
}
