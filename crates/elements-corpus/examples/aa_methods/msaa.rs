//! MSAA, the other candidate per kind (one-scene 1.2): the same
//! triangles filled solid, not moved, into a multisampled target that
//! is resolved, coverage from `SAMPLES` samples a pixel.

use vapors_gpu_harness::{Gpu, read_rgba8};
use vapors_shapes::MeshCorner;
use wgpu::util::DeviceExt as _;

use crate::cases::{Icon, PAGE, Placement, transform};
use crate::offscreen::{cleared_pass, pipeline_of, texture_for};
use crate::pipeline::{mesh, scale, size};

/// WebGPU guarantees 4 samples for an 8-bit RGBA target.
pub const SAMPLES: u32 = 4;

/// `icons` filled into a 4x MSAA target in `ink` under `placement`.
pub fn drawn(
    gpu: &Gpu,
    icons: &[&Icon],
    placement: &Placement,
    ink: [u8; 4],
    tolerance_px: f32,
) -> Vec<u8> {
    let viewport = size(icons.len(), placement);
    let floats = vertices(
        icons,
        placement,
        ink,
        tolerance_px,
        [viewport.width, viewport.height],
    );
    let bytes: Vec<u8> = floats.iter().flat_map(|f| f.to_le_bytes()).collect();
    let buffer = gpu
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("msaa vertices"),
            contents: &bytes,
            usage: wgpu::BufferUsages::VERTEX,
        });
    let (samples, resolved) = (
        texture_for(gpu, viewport, SAMPLES),
        texture_for(gpu, viewport, 1),
    );
    draw(
        gpu,
        (&samples, &resolved),
        &buffer,
        u32::try_from(floats.len() / 6).expect("vertices"),
    );
    read_rgba8(&gpu.device, &gpu.queue, &resolved)
}

/// Each icon's triangles in clip space, six floats a vertex: position
/// and colour.
fn vertices(
    icons: &[&Icon],
    placement: &Placement,
    ink: [u8; 4],
    tolerance_px: f32,
    size: [u32; 2],
) -> Vec<f32> {
    let colour = ink.map(|c| f32::from(c) / 255.0);
    let mut floats = Vec::new();
    for (at, icon) in (0u32..).zip(icons) {
        let m = transform(placement, at);
        for corner in mesh(icon, tolerance_px, scale(placement))
            .iter()
            .flat_map(|t| &t.corners)
        {
            floats.extend(clip(corner, m, size));
            floats.extend(colour);
        }
    }
    floats
}

fn clip(corner: &MeshCorner, m: [f32; 6], [width, height]: [u32; 2]) -> [f32; 2] {
    let [x, y] = corner.position;
    let pixel = [m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5]];
    [
        pixel[0] / width as f32 * 2.0 - 1.0,
        1.0 - pixel[1] / height as f32 * 2.0,
    ]
}

fn draw(
    gpu: &Gpu,
    (samples, resolved): (&wgpu::Texture, &wgpu::Texture),
    buffer: &wgpu::Buffer,
    count: u32,
) {
    let attributes = wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4];
    let pipeline = pipeline_of(
        &gpu.device,
        include_str!("msaa.wgsl"),
        (&attributes, 24),
        SAMPLES,
    );
    let (view, resolve) = (
        samples.create_view(&wgpu::TextureViewDescriptor::default()),
        resolved.create_view(&wgpu::TextureViewDescriptor::default()),
    );
    let mut encoder = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut pass = cleared_pass(&mut encoder, &view, Some(&resolve), PAGE);
        pass.set_pipeline(&pipeline);
        pass.set_vertex_buffer(0, buffer.slice(..));
        pass.draw(0..count, 0..1);
    }
    gpu.queue.submit([encoder.finish()]);
}
