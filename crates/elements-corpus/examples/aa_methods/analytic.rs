//! The analytic method (one-scene 1.2): each icon tessellated and drawn
//! by the mesh pipeline, its outline moved half a device pixel out and
//! coverage from the nearest outline edge's distance.

use vapors_gpu_harness::{Gpu, read_rgba8};
use vapors_renderer::{FrameEncoder, MeshEntries, MeshPipeline, MeshShape, Target, clear};
use vapors_scene::{Color, Size};
use vapors_shapes::{IconPath, MeshVertex, Paint, tessellate};

use crate::cases::{CELL, Icon, PAGE, Placement, transform};

/// `icon` stroked and tessellated within `tolerance_px` device pixels
/// at `scale` device pixels per unit.
pub fn mesh(icon: &Icon, tolerance_px: f32, scale: f32) -> Vec<[MeshVertex; 3]> {
    let path = IconPath::parse(icon.path).expect("touche's icons parse");
    let paint = if icon.stroke > 0.0 {
        Paint::Stroke { width: icon.stroke }
    } else {
        Paint::Fill(vapors_shapes::FillRule::NonZero)
    };
    tessellate(icon.name, &path, paint, tolerance_px / scale)
        .expect("touche's icons tessellate")
        .triangles
}

/// The page's size in device pixels for `count` cells under `placement`.
pub fn size(count: usize, placement: &Placement) -> Size {
    let count = u32::try_from(count).expect("a few cells");
    Size {
        width: CELL * count * placement.scale,
        height: CELL * placement.scale,
    }
}

/// `icons` drawn by the mesh pipeline in `ink` under `placement`.
pub fn drawn(
    gpu: &Gpu,
    icons: &[&Icon],
    placement: &Placement,
    ink: [u8; 4],
    tolerance_px: f32,
) -> Vec<u8> {
    let viewport = size(icons.len(), placement);
    let page = Color::from_rgba8(PAGE[0], PAGE[1], PAGE[2], PAGE[3]);
    let target = clear(
        &gpu.device,
        &gpu.queue,
        viewport.width,
        viewport.height,
        page,
    );
    let view = target.create_view(&wgpu::TextureViewDescriptor::default());
    let encoder = FrameEncoder::new(&gpu.device);
    let on = Target {
        device: &gpu.device,
        queue: &gpu.queue,
        encoder: &encoder,
        view: &view,
        viewport,
        scissor: None,
    };
    let mut entries = MeshEntries::new(&gpu.device);
    let meshes: Vec<_> = icons
        .iter()
        .map(|icon| mesh(icon, tolerance_px, scale(placement)))
        .collect();
    let written: Vec<_> = meshes
        .iter()
        .map(|m| entries.write(&on, m).expect("room"))
        .collect();
    let colour = Color::from_rgba8(ink[0], ink[1], ink[2], ink[3]);
    let items: Vec<_> = (0u32..)
        .zip(&written)
        .map(|(at, entry)| {
            let shape = MeshShape::new(transform(placement, at), colour)
                .expect("drawable")
                .expect("has area");
            (shape, entry)
        })
        .collect();
    MeshPipeline::new(&gpu.device, target.format()).draw(&on, &mut entries, &items);
    encoder.submit(&gpu.queue).expect("no draw refused");
    read_rgba8(&gpu.device, &gpu.queue, &target)
}

/// The device pixels per unit the placement draws at, for flattening:
/// the larger of its scale's axes.
pub fn scale(placement: &Placement) -> f32 {
    let [a, b, ..] = placement.linear;
    (a * a + b * b).sqrt() * placement.scale as f32
}
