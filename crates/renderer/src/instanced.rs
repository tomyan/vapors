//! A pipeline that draws one quad per instance: the shared machinery of
//! the box and shadow pipelines. Each instance is `vec4s` four-float
//! attributes, in device pixels where they are positions; the viewport
//! size is a uniform. Alpha blends over what the target holds, in
//! instance order, in one draw recorded into the target's encoder.

use vapors_scene::Size;

use crate::frame_arena::{FrameArena, Placed};
use crate::quads_setup::{bind_group_layout, render_pipeline};
use crate::{Target, WorkCounter};

const VERTICES_PER_QUAD: u32 = 6;

pub(crate) struct Instanced {
    label: &'static str,
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    viewports: FrameArena,
    instances: FrameArena,
    stride: u64,
    pub(crate) work: WorkCounter,
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
            viewports: FrameArena::uniforms(device, label),
            instances: FrameArena::new(device, label, wgpu::BufferUsages::VERTEX, stride),
            stride,
            work: WorkCounter::default(),
        }
    }

    /// Draws `count` instances whose attributes are `bytes`; a draw its
    /// arenas refuse is not recorded (the refusal is on the encoder).
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
        let (frame, queue) = (target.encoder, target.queue);
        let Some(instances) = self.instances.push(frame, queue, &self.work, bytes) else {
            return;
        };
        let viewport = viewport_bytes(target.viewport);
        let Some(viewport) = self.viewports.push(frame, queue, &self.work, &viewport) else {
            return;
        };
        self.record(target, count, &instances, &viewport);
    }

    fn record(&self, target: &Target<'_>, count: u32, instances: &Placed, viewport: &Placed) {
        let _encoding = tracing::trace_span!("draw step", phase = "encode").entered();
        let bind_group = target.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(self.label),
            layout: &self.layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: viewport.binding(),
            }],
        });
        target.draw_pass(|pass| {
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.set_vertex_buffer(0, instances.slice());
            self.work.draw(pass, 0..VERTICES_PER_QUAD, 0..count);
        });
    }
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
