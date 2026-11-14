//! A scene, its meshes and a renderer, rendered into a texture the size
//! of a surface backing and read back.

#![allow(dead_code)] // each test file uses part of it

use std::cell::RefCell;
use std::rc::Rc;
use vapors_app::{
    GpuWork, SURFACE_FORMAT, SurfaceError, SurfaceFrame, SurfaceRenderer, SurfaceSize, WorkCounter,
};
use vapors_gpu_harness::{Gpu, read_rgba8};
use vapors_scene3d::{
    CameraHandle, Colour, GeometryHandle, Meshes, NodeId, NodeKind, Projection, Scene, Transform,
    Vec3,
};
use vapors_scene3d_gpu::SceneRenderer;

/// The backing: not square, so an aspect mistake moves the pixels.
pub const SIZE: SurfaceSize = SurfaceSize {
    width: 160,
    height: 120,
    scale: 1.0,
};

pub const RED: [u8; 4] = [255, 0, 0, 255];
pub const GREEN: [u8; 4] = [0, 255, 0, 255];

pub struct Fixture {
    pub scene: Rc<RefCell<Scene>>,
    pub meshes: Rc<RefCell<Meshes>>,
}

impl Fixture {
    pub fn new() -> Self {
        Self {
            scene: Rc::new(RefCell::new(Scene::new())),
            meshes: Rc::new(RefCell::new(Meshes::new())),
        }
    }

    /// A camera at `eye` looking down -Z through `projection`, made active.
    pub fn camera(&self, projection: Projection, eye: Vec3) -> NodeId {
        let handle = CameraHandle::new();
        let mut scene = self.scene.borrow_mut();
        let root = scene.root();
        let kind = NodeKind::Camera { handle, projection };
        let node = scene
            .add(root, kind, Transform::from_translation(eye).unwrap())
            .unwrap();
        scene.set_camera(Some(handle));
        node
    }

    /// A square `2 * half` wide in the XY plane, centred on the origin.
    pub fn square(&self, half: f32) -> GeometryHandle {
        let corners = [(-half, -half), (half, -half), (half, half), (-half, half)];
        let positions = corners.map(|(x, y)| Vec3::new(x, y, 0.0)).to_vec();
        self.meshes
            .borrow_mut()
            .geometry(positions, vec![0, 1, 2, 0, 2, 3])
            .unwrap()
    }

    /// A mesh of `geometry` in `rgba` under `parent` at `local`.
    pub fn mesh(
        &self,
        parent: NodeId,
        geometry: &GeometryHandle,
        rgba: [u8; 4],
        local: Transform,
    ) -> NodeId {
        let [r, g, b, _] = rgba.map(|c| f32::from(c) / 255.0);
        let colour = Colour::srgb(r, g, b).unwrap();
        let geometry = geometry.clone();
        let kind = NodeKind::Mesh { geometry, colour };
        self.scene.borrow_mut().add(parent, kind, local).unwrap()
    }

    /// A group under the root at `local`.
    pub fn group(&self, local: Transform) -> NodeId {
        let root = self.root();
        self.scene
            .borrow_mut()
            .add(root, NodeKind::Group, local)
            .unwrap()
    }

    pub fn root(&self) -> NodeId {
        self.scene.borrow().root()
    }

    pub fn renderer(&self, gpu: &Gpu) -> SceneRenderer {
        let mut renderer = SceneRenderer::new(Rc::clone(&self.scene), Rc::clone(&self.meshes));
        renderer.mount(&gpu.device).expect("mounts");
        renderer.resize(&gpu.device, SIZE).expect("resizes");
        renderer
    }
}

/// One render into a fresh backing, read back as RGBA rows from the top.
pub fn render(renderer: &mut SceneRenderer, gpu: &Gpu) -> Result<Vec<u8>, SurfaceError> {
    Ok(render_counted(renderer, gpu)?.0)
}

/// As [`render`], with the work the renderer issued through
/// `SurfaceFrame::work`.
pub fn render_counted(
    renderer: &mut SceneRenderer,
    gpu: &Gpu,
) -> Result<(Vec<u8>, GpuWork), SurfaceError> {
    let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("scene3d test backing"),
        size: wgpu::Extent3d {
            width: SIZE.width,
            height: SIZE.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: SURFACE_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    let work = WorkCounter::default();
    renderer.render(SurfaceFrame {
        device: &gpu.device,
        queue: &gpu.queue,
        encoder: &mut encoder,
        view: &view,
        size: SIZE,
        work: &work,
    })?;
    gpu.queue.submit([encoder.finish()]);
    Ok((read_rgba8(&gpu.device, &gpu.queue, &texture), work.take()))
}

/// The centre of the pixels exactly `rgba`, in backing pixels from the
/// top left, and how many there are.
#[allow(clippy::cast_precision_loss)] // backings are at most 2^14 wide
pub fn centroid(pixels: &[u8], rgba: [u8; 4]) -> ([f32; 2], usize) {
    let width = SIZE.width as usize;
    let hits: Vec<(usize, usize)> = pixels
        .as_chunks::<4>()
        .0
        .iter()
        .enumerate()
        .filter(|(_, pixel)| **pixel == rgba)
        .map(|(i, _)| (i % width, i / width))
        .collect();
    let count = hits.len().max(1) as f32;
    let sum = hits.iter().fold([0.0, 0.0], |[x, y], (px, py)| {
        [x + *px as f32 + 0.5, y + *py as f32 + 0.5]
    });
    ([sum[0] / count, sum[1] / count], hits.len())
}

/// Normalised device coordinates to backing pixels from the top left.
#[allow(clippy::cast_precision_loss)]
pub fn to_pixel(ndc: Vec3) -> [f32; 2] {
    [
        f32::midpoint(ndc.x, 1.0) * SIZE.width as f32,
        (1.0 - ndc.y) / 2.0 * SIZE.height as f32,
    ]
}

/// The pixel at `(x, y)`.
pub fn at(pixels: &[u8], x: u32, y: u32) -> [u8; 4] {
    assert!(
        x < SIZE.width && y < SIZE.height,
        "({x}, {y}) is outside the backing"
    );
    let i = ((y * SIZE.width + x) * 4) as usize;
    [pixels[i], pixels[i + 1], pixels[i + 2], pixels[i + 3]]
}
