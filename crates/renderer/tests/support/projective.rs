//! Drawing closed-form shapes on a plane in a scene pass, white over
//! black on a square target with a depth attachment, and reading each
//! pixel's coverage back: its red channel, 0 to 1.

// each test crate including this uses a part of it
#![allow(dead_code)]

use glam::{Mat4, Vec3, Vec4};
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
    reds(&pixels_3d(side, shapes, (view_projection, world), black))
}

/// The RGBA bytes of `shapes` drawn as [`coverage_3d`] draws them, over
/// `background`.
pub fn pixels_3d(
    side: u32,
    shapes: &[ClosedFormShape],
    camera: (Mat4, Mat4),
    background: Color,
) -> Vec<u8> {
    let bytes = ProjectiveClosedForms::bytes(shapes);
    scene_pixels(
        side,
        camera,
        background,
        |gpu, layouts, formats, pass, bound| {
            let pipeline = ProjectiveClosedForms::new(&gpu.device, layouts, formats);
            let instances = buffer(gpu, wgpu::BufferUsages::VERTEX, &bytes);
            pipeline.record(pass, bound, instances.slice(..), &WorkCounter::default());
        },
    )
}

/// The RGBA bytes of a `side` square target cleared to `background`,
/// after `draw` records into a scene pass on it under `view_projection`
/// and a node at `world`.
pub fn scene_pixels<'a>(
    side: u32,
    (view_projection, world): (Mat4, Mat4),
    background: Color,
    draw: impl FnOnce(&Gpu, &ProjectiveLayouts, ScenePassFormats, &mut wgpu::RenderPass<'_>, Bound<'_>)
    + 'a,
) -> Vec<u8> {
    let gpu = gpu();
    let target = clear(&gpu.device, &gpu.queue, side, side, background);
    let layouts = ProjectiveLayouts::new(&gpu.device);
    let formats = ScenePassFormats {
        colour: target.format(),
        depth: DEPTH,
        bias: wgpu::DepthBiasState::default(),
    };
    let groups = bind_groups(&gpu, &layouts, side, view_projection, world);
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
        draw(&gpu, &layouts, formats, &mut pass, bound);
    }
    gpu.queue.submit([encoder.finish()]);
    read_rgba8(&gpu.device, &gpu.queue, &target)
}

/// The red channel of RGBA `pixels`, 0 to 1: coverage, white over black.
pub fn reds(pixels: &[u8]) -> Vec<f32> {
    pixels.chunks(4).map(|p| f32::from(p[0]) / 255.0).collect()
}

/// Where plane point `q` lands on a `side` square target, device pixels
/// (+Y down), under `view_projection` and `world`.
///
/// # Panics
/// When `q` is at or behind the eye, where a projection mirrors or has no
/// screen point: an oracle placing such a point would judge against a
/// wrong outline, so it refuses, naming the point.
pub fn on_screen(side: u32, (view_projection, world): (Mat4, Mat4), q: [f32; 2]) -> [f32; 2] {
    let clip = view_projection * world * glam::Vec4::new(q[0], q[1], 0.0, 1.0);
    assert!(
        clip.w > 0.0,
        "plane point {q:?} is at or behind the eye (clip w {}), so it has no screen point; fix: \
         give the oracle only points in front, or clip its outline at the near plane",
        clip.w
    );
    #[allow(clippy::cast_precision_loss)] // test sizes are small
    let side = side as f32;
    [
        (clip.x / clip.w * 0.5 + 0.5) * side,
        (0.5 - clip.y / clip.w * 0.5) * side,
    ]
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

pub fn buffer(gpu: &Gpu, usage: wgpu::BufferUsages, bytes: &[u8]) -> wgpu::Buffer {
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

/// Each pixel's coverage of the shape `inside` says (shape units), on a
/// `side` square target, drawn on the CPU: each of 8 x 8 sample points a
/// pixel is cast as a ray onto the node's plane (z = 0 in the node's
/// space) through the inverse of `camera`, taken back through the
/// inverse of `affine` (shape units to plane units) and tested. An
/// oracle that shares no code with the GPU stages.
pub fn cpu_coverage_8x(
    side: u32,
    inside: &dyn Fn([f32; 2]) -> bool,
    affine: [f32; 6],
    camera: (Mat4, Mat4),
) -> Vec<f32> {
    let sample = |p: [f32; 2]| shape_point(side, affine, camera, p).is_some_and(inside);
    (0..side * side)
        .map(|at| {
            let [left, top] = [to_f32(at % side), to_f32(at / side)];
            let offset = |k: u32| (to_f32(k) + 0.5) / 8.0;
            let hits = (0..64)
                .filter(|k| sample([left + offset(k % 8), top + offset(k / 8)]))
                .count();
            to_f32(u32::try_from(hits).expect("at most 64")) / 64.0
        })
        .collect()
}

/// The shape point under device point `p` of a `side` square target:
/// the ray through it cast onto the node's plane (z = 0 in the node's
/// space) through the inverse of `camera`, then taken back through the
/// inverse of `affine`; `None` where the ray meets the plane behind the
/// eye or past the far plane.
pub fn shape_point(
    side: u32,
    affine: [f32; 6],
    (view_projection, world): (Mat4, Mat4),
    [x, y]: [f32; 2],
) -> Option<[f32; 2]> {
    let to_plane = (view_projection * world).inverse();
    let ndc = [x / to_f32(side) * 2.0 - 1.0, 1.0 - y / to_f32(side) * 2.0];
    let near = to_plane.project_point3(Vec3::new(ndc[0], ndc[1], 0.0));
    let far = to_plane.project_point3(Vec3::new(ndc[0], ndc[1], 1.0));
    let along = near.z / (near.z - far.z);
    let hit = near + (far - near) * along;
    (0.0..=1.0)
        .contains(&along)
        .then(|| unaffine(affine, [hit.x, hit.y]))
}

/// A test-sized count as `f32`: exact below 2^24.
pub fn to_f32(count: u32) -> f32 {
    f32::from(u16::try_from(count).expect("a test size under 65,536"))
}

/// The centre of pixel `at` (row by row) of a `side` square target.
pub fn pixel_centre(at: usize, side: u32) -> [f32; 2] {
    let at = u32::try_from(at).expect("a test image");
    [to_f32(at % side) + 0.5, to_f32(at / side) + 0.5]
}

/// Plane point `plane` taken back through `affine` (matrix(a, b, c, d,
/// e, f), shape units to plane units) to shape units.
fn unaffine(affine: [f32; 6], plane: [f32; 2]) -> [f32; 2] {
    let [xx, xy, yx, yy, dx, dy] = affine;
    let det = xx * yy - xy * yx;
    let (u, v) = (plane[0] - dx, plane[1] - dy);
    [(yy * u - yx * v) / det, (xx * v - xy * u) / det]
}

/// The signed distance from `p` to a rect at `origin` of `size` with
/// circular corners of `radius`, negative inside.
pub fn rounded_rect_distance(p: [f32; 2], origin: [f32; 2], size: [f32; 2], radius: f32) -> f32 {
    let half = [size[0] / 2.0, size[1] / 2.0];
    let q = [
        (p[0] - origin[0] - half[0]).abs() - half[0] + radius,
        (p[1] - origin[1] - half[1]).abs() - half[1] + radius,
    ];
    q[0].max(0.0).hypot(q[1].max(0.0)) + q[0].max(q[1]).min(0.0) - radius
}
