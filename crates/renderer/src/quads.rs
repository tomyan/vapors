//! Drawing a run of the scene's quads in one instanced draw: element
//! backgrounds. Alpha blends over what is in the target, in order.

use vapors_scene::{Quad, Size};

const INSTANCE_FLOATS: usize = 8;
const INSTANCE_SIZE: u64 = (INSTANCE_FLOATS * 4) as u64;
const VERTICES_PER_QUAD: u32 = 6;

/// A pipeline that draws quads. Create once per target format.
pub struct QuadPipeline {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    viewport: wgpu::Buffer,
    instances: wgpu::Buffer,
}

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

impl QuadPipeline {
    /// Build the pipeline for targets of `format`.
    #[must_use]
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("quads"),
            source: wgpu::ShaderSource::Wgsl(include_str!("quads.wgsl").into()),
        });
        let layout = crate::quads_setup::bind_group_layout(device);
        Self {
            pipeline: crate::quads_setup::render_pipeline(device, &layout, &shader, format),
            layout,
            viewport: crate::quads_setup::uniform(device),
            instances: instance_buffer(device, 0),
        }
    }

    /// Draw `quads` into `target`, later quads over earlier ones, in one
    /// submit. Nothing is submitted for an empty slice.
    pub fn draw(&mut self, target: &Target<'_>, quads: &[Quad]) {
        if quads.is_empty() {
            return;
        }
        let bytes = instance_bytes(quads);
        if self.instances.size() < bytes.len() as u64 {
            self.instances = instance_buffer(target.device, bytes.len() as u64);
        }
        target.queue.write_buffer(&self.instances, 0, &bytes);
        target
            .queue
            .write_buffer(&self.viewport, 0, &viewport_bytes(target.viewport));
        self.submit(target, quad_count(quads));
    }

    fn submit(&self, target: &Target<'_>, count: u32) {
        let bind_group = target.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("quads"),
            layout: &self.layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: self.viewport.as_entire_binding(),
            }],
        });
        let mut encoder = target
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("quads"),
            });
        {
            let mut pass = crate::quads_setup::load_pass(&mut encoder, target.view);
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.set_vertex_buffer(0, self.instances.slice(..));
            pass.draw(0..VERTICES_PER_QUAD, 0..count);
        }
        target.queue.submit([encoder.finish()]);
    }
}

/// The instance buffer's vertex layout.
pub(crate) fn instance_layout() -> wgpu::VertexBufferLayout<'static> {
    const ATTRIBUTES: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4];
    wgpu::VertexBufferLayout {
        array_stride: INSTANCE_SIZE,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &ATTRIBUTES,
    }
}

fn quad_count(quads: &[Quad]) -> u32 {
    u32::try_from(quads.len()).unwrap_or_else(|_| {
        panic!(
            "{} quads in one batch; at most {} fit one draw",
            quads.len(),
            u32::MAX
        )
    })
}

fn instance_buffer(device: &wgpu::Device, size: u64) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("quad instances"),
        size: size.max(INSTANCE_SIZE * 64),
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn instance_bytes(quads: &[Quad]) -> Vec<u8> {
    quads
        .iter()
        .flat_map(|quad| {
            let r = quad.rect;
            let c = quad.background;
            [r.left, r.top, r.right, r.bottom, c.r, c.g, c.b, c.a]
        })
        .flat_map(f32::to_le_bytes)
        .collect()
}

#[allow(clippy::cast_precision_loss)] // viewports are far below 2^24 pixels
fn viewport_bytes(viewport: Size) -> Vec<u8> {
    [viewport.width as f32, viewport.height as f32, 0.0, 0.0]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect()
}
