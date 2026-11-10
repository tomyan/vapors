//! Track resources built once: uniform layout and bind group, solid meshes, world pass.
use super::{mesh, pipeline};
use crate::model::{Camera, RADIUS, ROAD, Race, STRAIGHT};
use wgpu::util::DeviceExt;
use wgpu::{CommandEncoder, Extent3d, RenderPass, TextureView};

pub(super) fn uniform_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("racer uniforms"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    })
}
pub(super) fn uniform_bind(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    uniform: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("racer"),
        layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform.as_entire_binding(),
        }],
    })
}
pub(super) fn solid_meshes(device: &wgpu::Device) -> (wgpu::Buffer, u32) {
    let mut mesh = mesh::scenery();
    mesh.vertices.extend(mesh::car().vertices);
    let bytes: Vec<u8> = mesh.vertices.iter().flat_map(|v| v.to_le_bytes()).collect();
    let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("racer solid meshes"),
        contents: &bytes,
        usage: wgpu::BufferUsages::VERTEX,
    });
    let count = u32::try_from(mesh.vertices.len() / 7).expect("mesh fits u32");
    (buffer, count)
}
pub(super) fn begin_world_pass<'e>(
    encoder: &'e mut CommandEncoder,
    view: &TextureView,
    depth: &TextureView,
) -> RenderPass<'e> {
    let color = [Some(color_attachment(view))];
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("racer world"),
        color_attachments: &color,
        depth_stencil_attachment: Some(depth_attachment(depth)),
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    })
}
fn color_attachment(view: &TextureView) -> wgpu::RenderPassColorAttachment<'_> {
    wgpu::RenderPassColorAttachment {
        view,
        depth_slice: None,
        resolve_target: None,
        ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
            store: wgpu::StoreOp::Store,
        },
    }
}
fn depth_attachment(view: &wgpu::TextureView) -> wgpu::RenderPassDepthStencilAttachment<'_> {
    wgpu::RenderPassDepthStencilAttachment {
        view,
        depth_ops: Some(wgpu::Operations {
            load: wgpu::LoadOp::Clear(1.0),
            store: wgpu::StoreOp::Discard,
        }),
        stencil_ops: None,
    }
}
pub(super) fn uniform_buffer(device: &wgpu::Device) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("racer frame"),
        size: 80,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}
/// The ground pipeline and the depth-tested solids pipeline, from one shader.
pub(super) fn pipelines(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    layout: &wgpu::BindGroupLayout,
) -> (wgpu::RenderPipeline, wgpu::RenderPipeline) {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("racer track"),
        source: wgpu::ShaderSource::Wgsl(include_str!("../track.wgsl").into()),
    });
    let ground = pipeline::create(device, format, layout, &shader, false);
    (
        ground,
        pipeline::create(device, format, layout, &shader, true),
    )
}
pub(super) fn depth_view(device: &wgpu::Device, size: Extent3d) -> TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("racer depth"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}
/// The 80-byte frame uniform: viewport and clock, car, then track and camera angle.
pub(super) fn frame_uniforms(extent: Extent3d, race: &Race, camera: Camera, brake: f32) -> Vec<u8> {
    let values = [
        viewport_values(extent, camera.zoom, race.elapsed).to_vec(),
        car_values(race, camera, brake).to_vec(),
        track_values(camera).to_vec(),
    ];
    values
        .concat()
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect()
}
fn viewport_values(extent: Extent3d, zoom: f32, elapsed: f32) -> [f32; 4] {
    #[allow(clippy::cast_precision_loss)] // viewport dimensions fit f32 exactly
    [extent.width as f32, extent.height as f32, zoom, elapsed]
}
fn car_values(race: &Race, camera: Camera, brake: f32) -> [f32; 7] {
    [
        race.position[0] + camera.offset[0],
        race.position[1] + camera.offset[1],
        race.position[0],
        race.position[1],
        race.heading,
        race.speed,
        brake,
    ]
}
fn track_values(camera: Camera) -> [f32; 9] {
    let (sin, cos) = (camera.rotation.sin(), camera.rotation.cos());
    [
        400.0,
        STRAIGHT,
        RADIUS,
        ROAD,
        camera.tilt,
        cos,
        sin,
        0.0,
        0.0,
    ]
}
