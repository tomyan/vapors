//! The mesh pipeline's wgpu objects: two vertex buffers, an entry's
//! vertices and the items' instances, and no face culling (the
//! tessellator's winding is not checked).

use std::mem::offset_of;

use vapors_shapes::MeshVertex;

/// Per vertex, at `MeshVertex`'s own field offsets: position, normal
/// (two floats each), three edge distances, and the triangle's three
/// edge normals (two floats each).
const VERTEX: [wgpu::VertexAttribute; 6] = [
    attribute(
        0,
        offset_of!(MeshVertex, position),
        wgpu::VertexFormat::Float32x2,
    ),
    attribute(
        1,
        offset_of!(MeshVertex, normal),
        wgpu::VertexFormat::Float32x2,
    ),
    attribute(
        2,
        offset_of!(MeshVertex, edges),
        wgpu::VertexFormat::Float32x3,
    ),
    attribute(
        3,
        offset_of!(MeshVertex, edge_normals),
        wgpu::VertexFormat::Float32x2,
    ),
    attribute(
        4,
        offset_of!(MeshVertex, edge_normals) + 8,
        wgpu::VertexFormat::Float32x2,
    ),
    attribute(
        5,
        offset_of!(MeshVertex, edge_normals) + 16,
        wgpu::VertexFormat::Float32x2,
    ),
];

/// Per instance: the linear part; translation and the inverse of the
/// determinant; colour.
const INSTANCE: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![
    6 => Float32x4, 7 => Float32x4, 8 => Float32x4
];

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
            source: wgpu::ShaderSource::Wgsl(include_str!("meshes.wgsl").into()),
        },
    );
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("meshes"),
        bind_group_layouts: &[Some(layout)],
        immediate_size: 0,
    });
    let buffers = buffers();
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

/// An entry's vertices, then the items' instances.
fn buffers() -> [Option<wgpu::VertexBufferLayout<'static>>; 2] {
    [
        Some(wgpu::VertexBufferLayout {
            array_stride: crate::mesh_entries::VERTEX_BYTES,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &VERTEX,
        }),
        Some(wgpu::VertexBufferLayout {
            array_stride: crate::meshes::INSTANCE_BYTES,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &INSTANCE,
        }),
    ]
}
