//! Ground and solid geometry, sharing a tilted camera under the compiled HUD.
mod mesh;
mod pipeline;
mod setup;
use crate::model::{Camera, Race};
use setup::{
    begin_world_pass, depth_view, frame_uniforms, pipelines, solid_meshes, uniform_bind,
    uniform_buffer, uniform_layout,
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
        let bytes = frame_uniforms(frame.texture.size(), race, camera, brake);
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
        self.depth = Some((size, depth_view(frame.device, size)));
    }
}
