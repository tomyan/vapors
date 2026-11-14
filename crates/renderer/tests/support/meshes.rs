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

/// How far device point `p` lies outside `mesh` drawn under
/// `transform`, device pixels; 0 inside. The outline is the mesh's own
/// outline edges (those not FAR), so the oracle needs no curve maths.
pub fn outside_by(mesh: &[[MeshVertex; 3]], m: [f32; 6], p: [f32; 2]) -> f32 {
    let to_device = |q: [f32; 2]| {
        [
            m[0] * q[0] + m[2] * q[1] + m[4],
            m[1] * q[0] + m[3] * q[1] + m[5],
        ]
    };
    let inside = mesh
        .iter()
        .any(|t| contains(t.map(|v| to_device(v.position)), p));
    if inside {
        return 0.0;
    }
    outline_segments(mesh)
        .map(|[q, r]| segment_distance(p, to_device(q), to_device(r)))
        .fold(f32::INFINITY, f32::min)
}

/// Asserts no pixel whose centre lies more than half a device pixel
/// (and a rasteriser's margin) outside `mesh` under `transform` has any
/// coverage.
pub fn assert_nothing_past_half_a_pixel(
    coverage: &[f32],
    mesh: &[[MeshVertex; 3]],
    transform: [f32; 6],
    what: &str,
) {
    for y in 0..SIDE {
        for x in 0..SIDE {
            let centre = [
                f32::from(u16::try_from(x).expect("small")) + 0.5,
                f32::from(u16::try_from(y).expect("small")) + 0.5,
            ];
            let out = outside_by(mesh, transform, centre);
            let found = coverage[(y * SIDE + x) as usize];
            assert!(
                out <= 0.52 || found <= 1.5 / 255.0,
                "{what}: pixel ({x}, {y}) is {out} px outside the outline but covered {found}"
            );
        }
    }
}

fn outline_segments(mesh: &[[MeshVertex; 3]]) -> impl Iterator<Item = [[f32; 2]; 2]> + '_ {
    mesh.iter().flat_map(|t| {
        (0..3).filter_map(move |edge| {
            let outline = t[0].edges[edge] < vapors_shapes::FAR_EDGE;
            outline.then(|| [t[(edge + 1) % 3].position, t[(edge + 2) % 3].position])
        })
    })
}

fn contains([p0, p1, p2]: [[f32; 2]; 3], p: [f32; 2]) -> bool {
    let side =
        |a: [f32; 2], b: [f32; 2]| (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0]);
    let (s0, s1, s2) = (side(p0, p1), side(p1, p2), side(p2, p0));
    (s0 >= 0.0 && s1 >= 0.0 && s2 >= 0.0) || (s0 <= 0.0 && s1 <= 0.0 && s2 <= 0.0)
}

fn segment_distance(p: [f32; 2], a: [f32; 2], b: [f32; 2]) -> f32 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let t = (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
    let (qx, qy) = (a[0] + t * dx - p[0], a[1] + t * dy - p[1]);
    (qx * qx + qy * qy).sqrt()
}
