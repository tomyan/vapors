//! Ground and solid geometry, sharing a tilted camera, drawn into the
//! HUD's `gpu-surface` (docs/gpu-surface.md) behind its controls.
mod mesh;
mod pipeline;
mod setup;
use crate::model::{Camera, Race};
use setup::{
    begin_world_pass, depth_view, frame_uniforms, pipelines, solid_meshes, uniform_bind,
    uniform_buffer, uniform_layout,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use vapors_app::{SURFACE_FORMAT, SurfaceError, SurfaceFrame, SurfaceRenderer, SurfaceSize};

/// What one render of the track shows; the app writes it before each frame.
#[derive(Clone, Default, PartialEq)]
pub(crate) struct Scene {
    pub(crate) race: Race,
    pub(crate) camera: Camera,
    pub(crate) brake: f32,
}

/// What the track's renderer has done, shared with whoever made it.
// RELAXED: all but `renders` are read only by the resize and unmount
// tests (gpu_track_resize_tests.rs, gpu_track_unmount_tests.rs) until the
// inspector reports them.
#[derive(Default)]
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) struct TrackLog {
    /// How many times it has rendered.
    pub(crate) renders: Cell<u64>,
    /// How many times its depth texture was allocated.
    pub(crate) resizes: Cell<u64>,
    /// The depth texture's size: width, height.
    pub(crate) depth: Cell<Option<[u32; 2]>>,
    /// The size of the last render.
    pub(crate) rendered: Cell<Option<SurfaceSize>>,
    /// The renderer, and with it every GPU resource it holds, was dropped.
    pub(crate) dropped: Cell<bool>,
}

/// The track's renderer, mounted on the surface named `track`.
pub(crate) struct Track {
    scene: Rc<RefCell<Scene>>,
    log: Rc<TrackLog>,
    resources: Option<Resources>,
    depth: Option<wgpu::TextureView>,
}

/// Made once, at mount.
struct Resources {
    pipeline: wgpu::RenderPipeline,
    solids: wgpu::RenderPipeline,
    vertices: wgpu::Buffer,
    vertex_count: u32,
    uniform: wgpu::Buffer,
    bind: wgpu::BindGroup,
}
impl Track {
    pub(crate) fn new(scene: Rc<RefCell<Scene>>, log: Rc<TrackLog>) -> Self {
        Self {
            scene,
            log,
            resources: None,
            depth: None,
        }
    }
}
impl SurfaceRenderer for Track {
    fn mount(&mut self, device: &wgpu::Device) -> Result<(), SurfaceError> {
        let layout = uniform_layout(device);
        let uniform = uniform_buffer(device);
        let (vertices, vertex_count) = solid_meshes(device);
        let (pipeline, solids) = pipelines(device, SURFACE_FORMAT, &layout);
        self.resources = Some(Resources {
            pipeline,
            solids,
            vertices,
            vertex_count,
            bind: uniform_bind(device, &layout, &uniform),
            uniform,
        });
        Ok(())
    }
    fn resize(&mut self, device: &wgpu::Device, size: SurfaceSize) -> Result<(), SurfaceError> {
        let depth = depth_view(device, extent(size));
        let texture = depth.texture();
        self.log
            .depth
            .set(Some([texture.width(), texture.height()]));
        self.depth = Some(depth);
        self.log.resizes.set(self.log.resizes.get() + 1);
        Ok(())
    }
    fn render(&mut self, frame: SurfaceFrame<'_>) -> Result<(), SurfaceError> {
        let (Some(resources), Some(depth)) = (&self.resources, &self.depth) else {
            return Err("the track rendered before mount and resize".into());
        };
        let scene = self.scene.borrow();
        let bytes = frame_uniforms(extent(frame.size), &scene.race, scene.camera, scene.brake);
        frame.queue.write_buffer(&resources.uniform, 0, &bytes);
        resources.record(frame.encoder, frame.view, depth);
        self.log.renders.set(self.log.renders.get() + 1);
        self.log.rendered.set(Some(frame.size));
        Ok(())
    }
}
impl Drop for Track {
    fn drop(&mut self) {
        self.log.dropped.set(true);
    }
}
impl Resources {
    fn record(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        depth: &wgpu::TextureView,
    ) {
        let mut pass = begin_world_pass(encoder, view, depth);
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind, &[]);
        pass.draw(0..3, 0..1);
        pass.set_pipeline(&self.solids);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.draw(0..self.vertex_count, 0..1);
    }
}
fn extent(size: SurfaceSize) -> wgpu::Extent3d {
    wgpu::Extent3d {
        width: size.width,
        height: size.height,
        depth_or_array_layers: 1,
    }
}
