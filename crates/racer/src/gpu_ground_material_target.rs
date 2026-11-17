//! The ground material test's target (`gpu_ground_material_tests.rs`):
//! an odd-sized square drawn into, and the pixels it samples; and a
//! renderer drawn at any size (`gpu_world_tests.rs`).

use vapors_app::{SURFACE_FORMAT, SurfaceFrame, SurfaceRenderer, SurfaceSize, WorkCounter};
use vapors_gpu_harness::{Gpu, read_rgba8};

/// Odd, so the centre pixel's centre is the camera's axis.
pub const SIDE: u32 = 65;

/// The RGBA at the centre and at the left, top and bottom edges' middles.
pub type Samples = [[u8; 4]; 4];

fn size() -> SurfaceSize {
    SurfaceSize {
        width: SIDE,
        height: SIDE,
        scale: 1.0,
    }
}

/// `renderer` mounted and sized to the `SIDE` square.
pub fn mount(renderer: &mut dyn SurfaceRenderer, gpu: &Gpu) {
    mount_at(renderer, gpu, size());
}

/// A draw of `renderer` into a fresh `SIDE` square: its RGBA.
pub fn drawn(renderer: &mut dyn SurfaceRenderer, gpu: &Gpu) -> Vec<u8> {
    drawn_at(renderer, gpu, size())
}

/// `renderer` mounted and sized to `size`.
pub fn mount_at(renderer: &mut dyn SurfaceRenderer, gpu: &Gpu, size: SurfaceSize) {
    renderer.mount(&gpu.device).expect("mounts");
    renderer.resize(&gpu.device, size).expect("resizes");
}

/// A draw of `renderer` into a fresh texture of `size`: its RGBA.
pub fn drawn_at(renderer: &mut dyn SurfaceRenderer, gpu: &Gpu, size: SurfaceSize) -> Vec<u8> {
    let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("ground material test"),
        size: wgpu::Extent3d {
            width: size.width,
            height: size.height,
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
    let frame = SurfaceFrame {
        device: &gpu.device,
        queue: &gpu.queue,
        encoder: &mut encoder,
        view: &view,
        size,
        work: &work,
    };
    renderer.render(frame).expect("renders");
    gpu.queue.submit([encoder.finish()]);
    read_rgba8(&gpu.device, &gpu.queue, &texture)
}

/// The centre pixel, then the left, top and bottom edges' middles: a
/// row and a column, so a picture mirrored either way differs.
pub fn sampled(pixels: &[u8]) -> Samples {
    let middle = SIDE / 2;
    [
        (middle, middle),
        (0, middle),
        (middle, 0),
        (middle, SIDE - 1),
    ]
    .map(|(x, y)| {
        let at = ((y * SIDE + x) * 4) as usize;
        pixels[at..at + 4].try_into().expect("four bytes")
    })
}
