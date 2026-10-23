//! A pipeline that draws one quad per instance: the shared machinery of
//! the box and shadow pipelines. Each instance is `vec4s` four-float
//! attributes, in device pixels where they are positions; the viewport
//! size is a uniform. Alpha blends over what the target holds, in
//! instance order, in one draw and one submit.

use vapors_scene::Size;

use crate::quads_setup::{bind_group_layout, load_pass, render_pipeline, uniform};

const VERTICES_PER_QUAD: u32 = 6;

/// Where a draw goes: the target view and its size in pixels.
pub struct Target<'a> {
    /// Device the pipeline was made on.
    pub device: &'a wgpu::Device,
    /// Queue to submit to.
    pub queue: &'a wgpu::Queue,
    /// The view drawn into; pixels under no quad keep what was there.
    pub view: &'a wgpu::TextureView,
    /// Size of the view in pixels.
    pub viewport: Size,
}

pub(crate) struct Instanced {
    label: &'static str,
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    viewport: wgpu::Buffer,
    instances: wgpu::Buffer,
    stride: u64,
}

impl Instanced {
    pub(crate) fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        label: &'static str,
        source: &str,
        vec4s: u32,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(label),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let layout = bind_group_layout(device);
        let stride = u64::from(vec4s) * 16;
        let attributes: Vec<wgpu::VertexAttribute> = (0..vec4s)
            .map(|i| wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x4,
                offset: u64::from(i) * 16,
                shader_location: i,
            })
            .collect();
        let instance = wgpu::VertexBufferLayout {
            array_stride: stride,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &attributes,
        };
        Self {
            label,
            pipeline: render_pipeline(device, &layout, &shader, format, instance),
            layout,
            viewport: uniform(device),
            instances: instance_buffer(device, label, stride * 64),
            stride,
        }
    }

    /// Draws `count` instances whose attributes are `bytes`.
    pub(crate) fn draw(&mut self, target: &Target<'_>, bytes: &[u8]) {
        let count = u32::try_from(bytes.len() as u64 / self.stride).unwrap_or_else(|_| {
            panic!(
                "{}: more instances in one batch than fit one draw",
                self.label
            )
        });
        if count == 0 {
            return;
        }
        if self.instances.size() < bytes.len() as u64 {
            self.instances = instance_buffer(target.device, self.label, bytes.len() as u64);
        }
        target.queue.write_buffer(&self.instances, 0, bytes);
        target
            .queue
            .write_buffer(&self.viewport, 0, &viewport_bytes(target.viewport));
        self.submit(target, count);
    }

    fn submit(&self, target: &Target<'_>, count: u32) {
        let bind_group = target.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(self.label),
            layout: &self.layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: self.viewport.as_entire_binding(),
            }],
        });
        let mut encoder = target
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some(self.label),
            });
        {
            let mut pass = load_pass(&mut encoder, target.view);
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.set_vertex_buffer(0, self.instances.slice(..));
            pass.draw(0..VERTICES_PER_QUAD, 0..count);
        }
        target.queue.submit([encoder.finish()]);
    }
}

fn instance_buffer(device: &wgpu::Device, label: &str, size: u64) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

#[allow(clippy::cast_precision_loss)] // viewports are far below 2^24 pixels
fn viewport_bytes(viewport: Size) -> Vec<u8> {
    [viewport.width as f32, viewport.height as f32, 0.0, 0.0]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect()
}

/// Little-endian bytes of `floats`, the instance buffer's layout.
pub(crate) fn float_bytes(floats: impl IntoIterator<Item = f32>) -> Vec<u8> {
    floats.into_iter().flat_map(f32::to_le_bytes).collect()
}
