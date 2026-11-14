//! The exact-corner candidate (one-scene S3b's row), measured as a
//! shader-only prototype: the mesh pipeline's move and the mesh as it
//! is, each triangle given its corners' positions, miter offsets and
//! convexity, coverage from each corner's wedge (`corners.wgsl`).

use std::collections::HashMap;

use vapors_gpu_harness::{Gpu, read_rgba8};
use vapors_shapes::MeshVertex;
use wgpu::util::DeviceExt as _;

use crate::analytic::{mesh, scale, size};
use crate::cases::{Icon, PAGE, Placement, transform};

/// Floats a vertex: ten four-float attributes but the first two pairs.
const FLOATS: usize = 36;

/// `icons` drawn by the prototype in `ink` under `placement`.
pub fn drawn(
    gpu: &Gpu,
    icons: &[&Icon],
    placement: &Placement,
    ink: [u8; 4],
    tolerance_px: f32,
) -> Vec<u8> {
    let viewport = size(icons.len(), placement);
    let view = [viewport.width as f32, viewport.height as f32];
    let mut floats = Vec::new();
    for (at, icon) in (0u32..).zip(icons) {
        let triangles = mesh(icon, tolerance_px, scale(placement));
        let item = Item {
            transform: transform(placement, at),
            view,
            colour: ink.map(|c| f32::from(c) / 255.0),
        };
        vertices(&triangles, &item, &mut floats);
    }
    let target = crate::offscreen::texture_for(gpu, viewport, 1);
    let count = u32::try_from(floats.len() / FLOATS).expect("vertices");
    draw(gpu, &target, &floats, count);
    read_rgba8(&gpu.device, &gpu.queue, &target)
}

/// What every vertex of one icon carries besides its own corner data.
struct Item {
    transform: [f32; 6],
    view: [f32; 2],
    colour: [f32; 4],
}

fn vertices(triangles: &[[MeshVertex; 3]], item: &Item, out: &mut Vec<f32>) {
    let flags = convexity(triangles);
    let [a, b, c, d, e, f] = item.transform;
    let inverse = 1.0 / (a * d - b * c);
    for triangle in triangles {
        let corners: Vec<f32> = triangle
            .iter()
            .flat_map(|v| [v.position[0], v.position[1], v.normal[0], v.normal[1]])
            .collect();
        let flag: Vec<f32> = triangle
            .iter()
            .map(|v| flag_of(v, &flags))
            .chain([0.0])
            .collect();
        for vertex in triangle {
            out.extend(vertex.position);
            out.extend(vertex.normal);
            out.extend(&corners);
            out.extend(&flag);
            out.extend([
                a,
                b,
                c,
                d,
                e,
                f,
                inverse,
                0.0,
                item.view[0],
                item.view[1],
                0.0,
                0.0,
            ]);
            out.extend(item.colour);
        }
    }
}

/// 1 for a convex outline corner, -1 for a reflex one, 0 inside.
fn flag_of(vertex: &MeshVertex, reflex: &HashMap<[u32; 2], bool>) -> f32 {
    if vertex.normal == [0.0, 0.0] {
        return 0.0;
    }
    if reflex[&key(vertex.position)] {
        -1.0
    } else {
        1.0
    }
}

/// Whether each outline vertex is reflex: the triangles round it span
/// more than half a turn.
fn convexity(triangles: &[[MeshVertex; 3]]) -> HashMap<[u32; 2], bool> {
    let mut angles: HashMap<[u32; 2], f32> = HashMap::new();
    for triangle in triangles {
        for k in 0..3 {
            let (p, q, r) = (
                triangle[k].position,
                triangle[(k + 1) % 3].position,
                triangle[(k + 2) % 3].position,
            );
            *angles.entry(key(p)).or_default() += angle(p, q, r);
        }
    }
    angles
        .into_iter()
        .map(|(at, sum)| (at, sum > std::f32::consts::PI + 1e-3))
        .collect()
}

fn key(p: [f32; 2]) -> [u32; 2] {
    p.map(|v| (v + 0.0).to_bits())
}

fn angle(p: [f32; 2], q: [f32; 2], r: [f32; 2]) -> f32 {
    let (u, v) = ([q[0] - p[0], q[1] - p[1]], [r[0] - p[0], r[1] - p[1]]);
    (u[0] * v[1] - u[1] * v[0])
        .abs()
        .atan2(u[0] * v[0] + u[1] * v[1])
}

fn draw(gpu: &Gpu, target: &wgpu::Texture, floats: &[f32], count: u32) {
    let bytes: Vec<u8> = floats.iter().flat_map(|f| f.to_le_bytes()).collect();
    let buffer = gpu
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("corners"),
            contents: &bytes,
            usage: wgpu::BufferUsages::VERTEX,
        });
    let attributes = wgpu::vertex_attr_array![
        0 => Float32x2, 1 => Float32x2, 2 => Float32x4, 3 => Float32x4, 4 => Float32x4,
        5 => Float32x4, 6 => Float32x4, 7 => Float32x4, 8 => Float32x4, 9 => Float32x4
    ];
    let stride = (FLOATS * 4) as u64;
    let pipeline = crate::offscreen::pipeline_of(
        &gpu.device,
        include_str!("corners.wgsl"),
        (&attributes, stride),
        1,
    );
    let view = target.create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut pass = crate::offscreen::cleared_pass(&mut encoder, &view, None, PAGE);
        pass.set_pipeline(&pipeline);
        pass.set_vertex_buffer(0, buffer.slice(..));
        pass.draw(0..count, 0..1);
    }
    gpu.queue.submit([encoder.finish()]);
}
