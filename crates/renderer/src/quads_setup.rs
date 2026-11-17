//! The quad pipeline's fixed wgpu objects: its layout and pipeline.

/// The layout of a pipeline that reads one uniform at a dynamic offset:
/// the viewport (and, for meshes, a shift).
pub(crate) fn bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("instanced"),
        entries: &[uniform_entry()],
    })
}

/// The layout of a pipeline over the view uniform and the transform
/// table, read in the vertex stage (`table.rs`): the image pipeline's
/// view group (`image.rs`).
pub(crate) fn table_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("on the transform table"),
        entries: &[uniform_entry(), table_entry()],
    })
}

/// The instanced pipelines' layout (`instanced.rs`): the view uniform,
/// the transform table, and the clip table, read in the fragment stage
/// (`clip.wgsl`).
pub(crate) fn tables_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    let clips = wgpu::BindGroupLayoutEntry {
        binding: 2,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ..table_entry()
    };
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("instanced on the tables"),
        entries: &[uniform_entry(), table_entry(), clips],
    })
}

/// The transform table: read in the vertex stage, and in the fragment
/// stage for a clip's frame (`clip.wgsl`).
fn table_entry() -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding: 1,
        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn uniform_entry() -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding: 0,
        // the fragment stage reads the batch's rows_base (clip.wgsl)
        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: true,
            min_binding_size: None,
        },
        count: None,
    }
}

pub(crate) fn render_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    shader: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
    instance: wgpu::VertexBufferLayout<'_>,
) -> wgpu::RenderPipeline {
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("instanced"),
        bind_group_layouts: &[Some(layout)],
        immediate_size: 0,
    });
    crate::pipelines::render_pipeline(
        device,
        &wgpu::RenderPipelineDescriptor {
            label: Some("instanced"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[Some(instance)],
            },
            fragment: Some(wgpu::FragmentState {
                module: shader,
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
