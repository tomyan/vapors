//! Ground and solid geometry, sharing a tilted camera under the compiled HUD.
mod mesh;
mod pipeline;
mod setup;
use crate::model::{Camera, RADIUS, ROAD, Race, STRAIGHT};
use setup::{
    begin_world_pass, pipelines, solid_meshes, uniform_bind, uniform_buffer, uniform_layout,
};
use vapors_platform::Frame;

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
        let layout = uniform_layout(device);
        let uniform = uniform_buffer(device);
        let (vertices, vertex_count) = solid_meshes(device);
        let (pipeline, solids) = pipelines(device, frame.texture.format(), &layout);
        Self {
            pipeline,
            solids,
            vertices,
            vertex_count,
            depth: None,
            bind: uniform_bind(device, &layout, &uniform),
            uniform,
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
        self.record_world(&mut encoder, view);
        frame.queue.submit([encoder.finish()]);
    }
    fn record_world(&self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        let depth = &self.depth.as_ref().expect("depth allocated").1;
        let mut pass = begin_world_pass(encoder, view, depth);
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind, &[]);
        pass.draw(0..3, 0..1);
        pass.set_pipeline(&self.solids);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.draw(0..self.vertex_count, 0..1);
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
