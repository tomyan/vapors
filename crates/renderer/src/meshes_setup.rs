//! The mesh pipeline's wgpu objects: two vertex buffers, both stepped
//! per instance, an entry's triangles (one instance each, the shader
//! choosing the corner by vertex index) and the item's instance (stride
//! 0, so every triangle of a draw reads it); no face culling (the
//! tessellator's winding is not checked).

use std::mem::offset_of;

use vapors_shapes::{MeshCorner, MeshTriangle};

/// Per triangle, at `MeshTriangle`'s own field offsets, for each corner
/// `k` at locations `4k` to `4k + 3`: its position (two floats), its two
/// normals (four), its kind (one) and its two reaches (two).
const TRIANGLE: [wgpu::VertexAttribute; 12] = [
    position(0),
    normals(0),
    kind(0),
    reaches(0),
    position(1),
    normals(1),
    kind(1),
    reaches(1),
    position(2),
    normals(2),
    kind(2),
    reaches(2),
];

/// Per item: the linear part; translation and the inverse of the
/// determinant; colour.
const ITEM: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![
    12 => Float32x4, 13 => Float32x4, 14 => Float32x4
];

/// The page's item: [`ITEM`], then its row's index and its frame's entry
/// in the transform table (`meshes.rs` `bytes_in`) as one attribute, the
/// triangle and item taking 15 of the 16 locations WebGPU's default
/// limits give.
const PAGE_ITEM: [wgpu::VertexAttribute; 4] = wgpu::vertex_attr_array![
    12 => Float32x4, 13 => Float32x4, 14 => Float32x4, 15 => Uint32x2
];

const fn corner(k: usize) -> usize {
    offset_of!(MeshTriangle, corners) + k * size_of::<MeshCorner>()
}

#[allow(clippy::cast_possible_truncation)] // k < 3
const fn position(k: usize) -> wgpu::VertexAttribute {
    attribute(
        4 * k as u32,
        corner(k) + offset_of!(MeshCorner, position),
        wgpu::VertexFormat::Float32x2,
    )
}

#[allow(clippy::cast_possible_truncation)] // k < 3
const fn normals(k: usize) -> wgpu::VertexAttribute {
    attribute(
        4 * k as u32 + 1,
        corner(k) + offset_of!(MeshCorner, normals),
        wgpu::VertexFormat::Float32x4,
    )
}

#[allow(clippy::cast_possible_truncation)] // k < 3
const fn reaches(k: usize) -> wgpu::VertexAttribute {
    attribute(
        4 * k as u32 + 3,
        corner(k) + offset_of!(MeshCorner, reaches),
        wgpu::VertexFormat::Float32x2,
    )
}

#[allow(clippy::cast_possible_truncation)] // k < 3
const fn kind(k: usize) -> wgpu::VertexAttribute {
    attribute(
        4 * k as u32 + 2,
        corner(k) + offset_of!(MeshCorner, kind),
        wgpu::VertexFormat::Float32,
    )
}

const fn attribute(
    location: u32,
    offset: usize,
    format: wgpu::VertexFormat,
) -> wgpu::VertexAttribute {
    wgpu::VertexAttribute {
        format,
        offset: offset as u64,
        shader_location: location,
    }
}

pub(crate) fn render_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let shader = crate::pipelines::shader_module(
        device,
        wgpu::ShaderModuleDescriptor {
            label: Some("meshes"),
            source: wgpu::ShaderSource::Wgsl(
                concat!(
                    include_str!("transform_table.wgsl"),
                    include_str!("meshes_2d.wgsl"),
                    include_str!("meshes_shared.wgsl")
                )
                .into(),
            ),
        },
    );
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("meshes"),
        bind_group_layouts: &[Some(layout)],
        immediate_size: 0,
    });
    let buffers = page_buffers();
    crate::pipelines::render_pipeline(
        device,
        &wgpu::RenderPipelineDescriptor {
            label: Some("meshes"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &buffers,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        },
    )
}

/// An entry's triangles, then the item, at stride 0.
/// The page's buffers: [`buffers`], the item with its row and frame.
fn page_buffers() -> [Option<wgpu::VertexBufferLayout<'static>>; 2] {
    let [triangles, _] = buffers();
    [
        triangles,
        Some(wgpu::VertexBufferLayout {
            array_stride: 0,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &PAGE_ITEM,
        }),
    ]
}

pub(crate) fn buffers() -> [Option<wgpu::VertexBufferLayout<'static>>; 2] {
    [
        Some(wgpu::VertexBufferLayout {
            array_stride: crate::mesh_entries::TRIANGLE_BYTES,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &TRIANGLE,
        }),
        Some(wgpu::VertexBufferLayout {
            array_stride: 0,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &ITEM,
        }),
    ]
}
