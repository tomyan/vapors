//! Drawing tessellated paths through the mesh pipeline white over black
//! on a 32 x 32 target and reading each pixel's coverage back.

// each test crate including this uses a part of it
#![allow(dead_code)]

use vapors_gpu_harness::{gpu, read_rgba8};
use vapors_renderer::{FrameEncoder, MeshEntries, MeshPipeline, MeshShape, Target, clear};
use vapors_scene::{Color, Size};
use vapors_shapes::{FillRule, IconPath, MeshVertex, Paint, tessellate};

pub const SIDE: u32 = 32;

/// A mesh's triangles, its transform and its colour.
pub type Drawn = (Vec<[MeshVertex; 3]>, [f32; 6], Color);

/// `data` filled (nonzero) or stroked `stroke` wide, tessellated.
pub fn mesh(data: &str, stroke: Option<f32>) -> Vec<[MeshVertex; 3]> {
    let path = IconPath::parse(data).unwrap_or_else(|e| panic!("{data:?}: {e}"));
    let paint = stroke.map_or(Paint::Fill(FillRule::NonZero), |width| Paint::Stroke {
        width,
    });
    tessellate("a.wisp:1", &path, paint, 0.05)
        .unwrap_or_else(|e| panic!("{data:?}: {e}"))
        .triangles
}

/// Each pixel's coverage, row by row, of `meshes` drawn in order, each
/// under its transform in its colour.
pub fn coverage(meshes: &[Drawn]) -> Vec<f32> {
    let gpu = gpu();
    let target = clear(
        &gpu.device,
        &gpu.queue,
        SIDE,
        SIDE,
        Color::from_rgba8(0, 0, 0, 255),
    );
    let view = target.create_view(&wgpu::TextureViewDescriptor::default());
    let encoder = FrameEncoder::new(&gpu.device);
    let viewport = Size {
        width: SIDE,
        height: SIDE,
    };
    let (device, queue) = (&gpu.device, &gpu.queue);
    let on = Target {
        device,
        queue,
        encoder: &encoder,
        view: &view,
        viewport,
        scissor: None,
    };
    let mut entries = MeshEntries::new(device);
    let written: Vec<_> = meshes
        .iter()
        .map(|(triangles, _, _)| entries.write(&on, triangles).expect("room"))
        .collect();
    let items: Vec<_> = meshes
        .iter()
        .zip(&written)
        .map(|((_, transform, colour), entry)| {
            let shape = MeshShape::new(*transform, *colour)
                .expect("drawable")
                .expect("has area");
            (shape, entry)
        })
        .collect();
    MeshPipeline::new(device, target.format()).draw(&on, &mut entries, &items);
    encoder.submit(queue).expect("no draw refused");
    let pixels = read_rgba8(device, queue, &target);
    pixels.chunks(4).map(|p| f32::from(p[0]) / 255.0).collect()
}

/// `mesh` in white under `transform`.
pub fn white(
    mesh: Vec<[MeshVertex; 3]>,
    transform: [f32; 6],
) -> (Vec<[MeshVertex; 3]>, [f32; 6], Color) {
    (mesh, transform, Color::from_rgba8(255, 255, 255, 255))
}

pub fn scaled(scale: f32) -> [f32; 6] {
    [scale, 0.0, 0.0, scale, 0.0, 0.0]
}

/// Pixel (`x`, `y`)'s coverage, within one 8-bit step and a little of
/// the rasteriser's rounding of `expected`.
pub fn assert_at(coverage: &[f32], (x, y): (u32, u32), expected: f32, what: &str) {
    let found = coverage[(y * SIDE + x) as usize];
    assert!(
        (found - expected).abs() <= 1.5 / 255.0,
        "{what}: pixel ({x}, {y}) coverage {found}, expected {expected}"
    );
}
