//! Drawing tessellated paths through the mesh pipeline white over black
//! on a 32 x 32 target and reading each pixel's coverage back.

// each test crate including this uses a part of it
#![allow(dead_code)]

use vapors_gpu_harness::{gpu, read_rgba8};
use vapors_renderer::{FrameEncoder, MeshEntries, MeshPipeline, MeshShape, Target, clear};
use vapors_scene::{Color, Size};
use vapors_shapes::{FillRule, IconPath, MeshTriangle, Paint, tessellate};

pub const SIDE: u32 = 32;

/// A mesh's triangles, its transform and its colour.
pub type Drawn = (Vec<MeshTriangle>, [f32; 6], Color);

/// `data` filled (nonzero) or stroked `stroke` wide, tessellated.
pub fn mesh(data: &str, stroke: Option<f32>) -> Vec<MeshTriangle> {
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
pub fn white(mesh: Vec<MeshTriangle>, transform: [f32; 6]) -> (Vec<MeshTriangle>, [f32; 6], Color) {
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

/// Where a mesh's points land on the device: any map that keeps lines
/// straight (an affine map, or a plane's projection).
pub type Place<'a> = &'a dyn Fn([f32; 2]) -> [f32; 2];

/// How far device point `p` lies outside `mesh`, its corners placed by
/// `place`, device pixels; 0 inside. The outline is the mesh's own
/// outline edges, so the oracle needs no curve maths.
pub fn outside_by(mesh: &[MeshTriangle], place: Place<'_>, p: [f32; 2]) -> f32 {
    if covers(mesh, place, p) {
        return 0.0;
    }
    to_outline(mesh, place, p)
}

/// How far device point `p` lies inside `mesh`, placed as
/// [`outside_by`] places it, device pixels; 0 outside.
pub fn inside_by(mesh: &[MeshTriangle], place: Place<'_>, p: [f32; 2]) -> f32 {
    if !covers(mesh, place, p) {
        return 0.0;
    }
    to_outline(mesh, place, p)
}

fn covers(mesh: &[MeshTriangle], place: Place<'_>, p: [f32; 2]) -> bool {
    mesh.iter()
        .any(|t| contains(t.corners.map(|c| place(c.position)), p))
}

fn to_outline(mesh: &[MeshTriangle], place: Place<'_>, p: [f32; 2]) -> f32 {
    outline_segments(mesh)
        .map(|[q, r]| segment_distance(p, place(q), place(r)))
        .fold(f32::INFINITY, f32::min)
}

/// Both halves of the rule for `mesh` under `transform`: nothing covered
/// past half a device pixel outside the outline, everything past half a
/// pixel inside it.
pub fn assert_ramp_within_half_a_pixel(
    coverage: &[f32],
    mesh: &[MeshTriangle],
    transform: [f32; 6],
    what: &str,
) {
    assert_ramp_within_half_a_pixel_placed(coverage, mesh, &|q| to_device(transform, q), what);
}

/// [`assert_ramp_within_half_a_pixel`] with the corners placed by
/// `place`. Each pixel's centre is judged, with a rasteriser's margin of
/// 0.02 px.
pub fn assert_ramp_within_half_a_pixel_placed(
    coverage: &[f32],
    mesh: &[MeshTriangle],
    place: Place<'_>,
    what: &str,
) {
    for (x, y, centre) in pixel_centres() {
        let found = coverage[(y * SIDE + x) as usize];
        let out = outside_by(mesh, place, centre);
        assert!(
            out <= 0.52 || found <= 1.5 / 255.0,
            "{what}: pixel ({x}, {y}) is {out} px outside the outline but covered {found}"
        );
        let depth = inside_by(mesh, place, centre);
        assert!(
            depth < 0.52 || found >= 254.0 / 255.0,
            "{what}: pixel ({x}, {y}) is {depth} px inside the outline but covered only {found}"
        );
    }
}

pub fn pixel_centres() -> impl Iterator<Item = (u32, u32, [f32; 2])> {
    (0..SIDE).flat_map(|y| {
        (0..SIDE).map(move |x| {
            let at = |v: u32| f32::from(u16::try_from(v).expect("small")) + 0.5;
            (x, y, [at(x), at(y)])
        })
    })
}

pub fn to_device(m: [f32; 6], q: [f32; 2]) -> [f32; 2] {
    [
        m[0] * q[0] + m[2] * q[1] + m[4],
        m[1] * q[0] + m[3] * q[1] + m[5],
    ]
}

pub fn outline_segments(mesh: &[MeshTriangle]) -> impl Iterator<Item = [[f32; 2]; 2]> + '_ {
    mesh.iter().flat_map(|t| {
        (0..3)
            .filter(|&edge| t.on_outline(edge))
            .map(|edge| [1, 2].map(|k| t.corners[(edge + k) % 3].position))
    })
}

pub fn contains([p0, p1, p2]: [[f32; 2]; 3], p: [f32; 2]) -> bool {
    let side =
        |a: [f32; 2], b: [f32; 2]| (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0]);
    let (s0, s1, s2) = (side(p0, p1), side(p1, p2), side(p2, p0));
    (s0 >= 0.0 && s1 >= 0.0 && s2 >= 0.0) || (s0 <= 0.0 && s1 <= 0.0 && s2 <= 0.0)
}

pub fn segment_distance(p: [f32; 2], a: [f32; 2], b: [f32; 2]) -> f32 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let t = (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
    let (qx, qy) = (a[0] + t * dx - p[0], a[1] + t * dy - p[1]);
    (qx * qx + qy * qy).sqrt()
}
