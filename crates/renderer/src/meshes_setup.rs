//! The mesh pipeline's wgpu objects: two vertex buffers, an entry's
//! vertices and the items' instances, and no face culling (the
//! tessellator's winding is not checked).

/// Per vertex: position, normal (two floats each), three edge
/// distances, three shifts.
const VERTEX: [wgpu::VertexAttribute; 4] = wgpu::vertex_attr_array![
    0 => Float32x2, 1 => Float32x2, 2 => Float32x3, 3 => Float32x3
];

/// Per instance: the linear part; translation and extrusion; colour.
const INSTANCE: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![
    4 => Float32x4, 5 => Float32x4, 6 => Float32x4
];

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
            array_stride: 48,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &INSTANCE,
        }),
    ]
}
