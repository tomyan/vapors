//! One full-screen GPU pass for the circuit and car, under the Vapors HUD.
use crate::model::{Camera, RADIUS, ROAD, Race, STRAIGHT};
use vapors_platform::Frame;

pub(crate) struct Track {
    pipeline: wgpu::RenderPipeline,
    uniform: wgpu::Buffer,
    bind: wgpu::BindGroup,
}
impl Track {
    pub(crate) fn new(frame: &Frame<'_>) -> Self {
        let device = frame.device;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("racer track"),
            source: wgpu::ShaderSource::Wgsl(include_str!("track.wgsl").into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("racer uniforms"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("racer frame"),
            size: 64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("racer"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        });
        Self {
            pipeline: pipeline(device, frame.texture.format(), &layout, &shader),
            uniform,
            bind,
        }
    }
    pub(crate) fn draw(
        &self,
        frame: &Frame<'_>,
        view: &wgpu::TextureView,
        race: &Race,
        camera: Camera,
        brake: f32,
    ) {
        let extent = frame.texture.size();
        let look = race.speed * 0.20;
        #[allow(clippy::cast_precision_loss)] // viewport dimensions fit f32 exactly
        let values = [
            extent.width as f32,
            extent.height as f32,
            camera.zoom,
            race.elapsed,
            race.position[0] + race.heading.cos() * look,
            race.position[1] + race.heading.sin() * look,
            race.position[0],
            race.position[1],
            race.heading,
            race.speed,
            brake,
            0.0,
            STRAIGHT,
            RADIUS,
            ROAD,
            0.0,
        ];
        let bytes: Vec<u8> = values.iter().flat_map(|v| v.to_le_bytes()).collect();
        frame.queue.write_buffer(&self.uniform, 0, &bytes);
        self.submit(frame, view);
    }
    fn submit(&self, frame: &Frame<'_>, view: &wgpu::TextureView) {
        let mut encoder = frame
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("racer"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("racer world"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind, &[]);
            pass.draw(0..3, 0..1);
        }
        frame.queue.submit([encoder.finish()]);
    }
}
fn pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind: &wgpu::BindGroupLayout,
    shader: &wgpu::ShaderModule,
) -> wgpu::RenderPipeline {
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("racer"),
        bind_group_layouts: &[Some(bind)],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("racer"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}
