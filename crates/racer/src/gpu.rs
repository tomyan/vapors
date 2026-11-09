//! Ground and solid geometry, sharing a tilted camera under the compiled HUD.
mod mesh;
mod pipeline;
use crate::model::{Camera, RADIUS, ROAD, Race, STRAIGHT};
use vapors_platform::Frame;
use wgpu::util::DeviceExt;

pub(crate) struct Track {
    pipeline: wgpu::RenderPipeline,
    solids: wgpu::RenderPipeline,
    vertices: wgpu::Buffer,
    vertex_count: u32,
    depth: Option<(wgpu::Extent3d, wgpu::TextureView)>,
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
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
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
            size: 80,
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
        let mut mesh = mesh::scenery();
        mesh.vertices.extend(mesh::car().vertices);
        let bytes: Vec<u8> = mesh.vertices.iter().flat_map(|v| v.to_le_bytes()).collect();
        let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("racer solid meshes"),
            contents: &bytes,
            usage: wgpu::BufferUsages::VERTEX,
        });
        Self {
            pipeline: pipeline::create(device, frame.texture.format(), &layout, &shader, false),
            solids: pipeline::create(device, frame.texture.format(), &layout, &shader, true),
            vertices,
            vertex_count: u32::try_from(mesh.vertices.len() / 7).expect("mesh fits u32"),
            depth: None,
            uniform,
            bind,
        }
    }
    pub(crate) fn draw(
        &mut self,
        frame: &Frame<'_>,
        view: &wgpu::TextureView,
        race: &Race,
        camera: Camera,
        brake: f32,
    ) {
        let extent = frame.texture.size();
        #[allow(clippy::cast_precision_loss)] // viewport dimensions fit f32 exactly
        let values = [
            extent.width as f32,
            extent.height as f32,
            camera.zoom,
            race.elapsed,
            race.position[0] + camera.offset[0],
            race.position[1] + camera.offset[1],
            race.position[0],
            race.position[1],
            race.heading,
            race.speed,
            brake,
            400.0,
            STRAIGHT,
            RADIUS,
            ROAD,
            camera.tilt,
            camera.rotation.cos(),
            camera.rotation.sin(),
            0.0,
            0.0,
        ];
        let bytes: Vec<u8> = values.iter().flat_map(|v| v.to_le_bytes()).collect();
        frame.queue.write_buffer(&self.uniform, 0, &bytes);
        self.submit(frame, view);
    }
    fn submit(&mut self, frame: &Frame<'_>, view: &wgpu::TextureView) {
        self.resize_depth(frame);
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
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth.as_ref().expect("depth allocated").1,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind, &[]);
            pass.draw(0..3, 0..1);
            pass.set_pipeline(&self.solids);
            pass.set_vertex_buffer(0, self.vertices.slice(..));
            pass.draw(0..self.vertex_count, 0..1);
        }
        frame.queue.submit([encoder.finish()]);
    }
    fn resize_depth(&mut self, frame: &Frame<'_>) {
        let size = frame.texture.size();
        if self.depth.as_ref().is_some_and(|(old, _)| *old == size) {
            return;
        }
        let texture = frame.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("racer depth"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        self.depth = Some((
            size,
            texture.create_view(&wgpu::TextureViewDescriptor::default()),
        ));
    }
}
