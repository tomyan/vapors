//! Drawing closed-form shapes on a plane in a scene pass, white over
//! black on a square target with a depth attachment, and reading each
//! pixel's coverage back: its red channel, 0 to 1.

// each test crate including this uses a part of it
#![allow(dead_code)]

use glam::{Mat4, Vec4};
use vapors_gpu_harness::{Gpu, gpu, read_rgba8};
use vapors_renderer::projective::ProjectiveLayouts;
use vapors_renderer::{
    Bound, ClosedFormShape, ProjectiveClosedForms, ScenePassFormats, WorkCounter, clear,
};
use vapors_scene::Color;

const DEPTH: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// A camera that puts plane unit (x, y) at device pixel (x, y) of a
/// `side` square target, +Y down, as the page's shapes are placed.
pub fn pixel_camera(side: u32) -> Mat4 {
    #[allow(clippy::cast_precision_loss)] // test sizes are small
    let side = side as f32;
    Mat4::from_cols(
        Vec4::new(2.0 / side, 0.0, 0.0, 0.0),
        Vec4::new(0.0, -2.0 / side, 0.0, 0.0),
        Vec4::new(0.0, 0.0, 1.0, 0.0),
        Vec4::new(-1.0, 1.0, 0.5, 1.0),
    )
}

/// Each pixel's coverage of `shapes` on the plane `world` places, seen
/// by `view_projection`, on a `side` square target.
pub fn coverage_3d(
    side: u32,
    shapes: &[ClosedFormShape],
    view_projection: Mat4,
    world: Mat4,
) -> Vec<f32> {
    let black = Color::from_rgba8(0, 0, 0, 255);
    let pixels = pixels_3d(side, shapes, (view_projection, world), black);
    pixels.chunks(4).map(|p| f32::from(p[0]) / 255.0).collect()
}

/// The RGBA bytes of `shapes` drawn as [`coverage_3d`] draws them, over
/// `background`.
pub fn pixels_3d(
    side: u32,
    shapes: &[ClosedFormShape],
    (view_projection, world): (Mat4, Mat4),
    background: Color,
) -> Vec<u8> {
    let gpu = gpu();
    let target = clear(&gpu.device, &gpu.queue, side, side, background);
    let layouts = ProjectiveLayouts::new(&gpu.device);
    let formats = ScenePassFormats {
        colour: target.format(),
        depth: DEPTH,
        bias: wgpu::DepthBiasState::default(),
    };
    let pipeline = ProjectiveClosedForms::new(&gpu.device, &layouts, formats);
    let groups = bind_groups(&gpu, &layouts, side, view_projection, world);
    let instances = buffer(
        &gpu,
        wgpu::BufferUsages::VERTEX,
        &ProjectiveClosedForms::bytes(shapes),
    );
    let colour = target.create_view(&wgpu::TextureViewDescriptor::default());
    let depth = depth_view(&gpu, side);
    let mut encoder = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut pass = begin(&mut encoder, &colour, &depth);
        let bound = Bound {
            camera: &groups.0,
            node: &groups.1,
            offset: 0,
        };
        pipeline.record(
            &mut pass,
            bound,
            instances.slice(..),
            &WorkCounter::default(),
        );
    }
    gpu.queue.submit([encoder.finish()]);
    read_rgba8(&gpu.device, &gpu.queue, &target)
}

/// `fine` (a square image `factor` times `side` wide) averaged down to
/// `side` square: an 8x reference box-filtered.
pub fn downsampled(fine: &[f32], side: u32, factor: u32) -> Vec<f32> {
    let wide = (side * factor) as usize;
    let (side, factor) = (side as usize, factor as usize);
    #[allow(clippy::cast_precision_loss)] // factor is small
    let samples = (factor * factor) as f32;
    let mut out = vec![0.0; side * side];
    for (at, value) in fine.iter().enumerate() {
        out[(at / wide / factor) * side + (at % wide) / factor] += value / samples;
    }
    out
}

fn bind_groups(
    gpu: &Gpu,
    layouts: &ProjectiveLayouts,
    side: u32,
    view_projection: Mat4,
    world: Mat4,
) -> (wgpu::BindGroup, wgpu::BindGroup) {
    #[allow(clippy::cast_precision_loss)] // test sizes are small
    let extent = side as f32;
    let mut camera = view_projection.to_cols_array().to_vec();
    camera.extend([extent, extent, 0.0, 0.0]);
    let mut node = world.to_cols_array().to_vec();
    node.extend([1.0; 4]);
    let group = |layout, floats: &[f32]| {
        let bytes: Vec<u8> = floats.iter().flat_map(|f| f.to_le_bytes()).collect();
        let uniform = buffer(gpu, wgpu::BufferUsages::UNIFORM, &bytes);
        gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &uniform,
                    offset: 0,
                    size: wgpu::BufferSize::new(80),
                }),
            }],
        })
    };
    (group(&layouts.camera, &camera), group(&layouts.node, &node))
}

fn buffer(gpu: &Gpu, usage: wgpu::BufferUsages, bytes: &[u8]) -> wgpu::Buffer {
    let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: bytes.len() as u64,
        usage: usage | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    gpu.queue.write_buffer(&buffer, 0, bytes);
    buffer
}

fn depth_view(gpu: &Gpu, side: u32) -> wgpu::TextureView {
    let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("scene depth"),
        size: wgpu::Extent3d {
            width: side,
            height: side,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

fn begin<'e>(
    encoder: &'e mut wgpu::CommandEncoder,
    colour: &wgpu::TextureView,
    depth: &wgpu::TextureView,
) -> wgpu::RenderPass<'e> {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("scene shapes test"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: colour,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            view: depth,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(1.0),
                store: wgpu::StoreOp::Discard,
            }),
            stencil_ops: None,
        }),
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    })
}
