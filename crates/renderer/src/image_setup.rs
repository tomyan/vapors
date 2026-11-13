//! The image pipeline's bind group layouts and render pipeline.

use crate::quad::Vertex;

pub(crate) fn bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("image"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    })
}

/// The nearest and smooth samplers.
pub(crate) fn samplers(device: &wgpu::Device) -> (wgpu::Sampler, wgpu::Sampler) {
    let sampler = |label, filter, mipmap_filter| {
        device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some(label),
            mag_filter: filter,
            min_filter: filter,
            mipmap_filter,
            ..Default::default()
        })
    };
    let (linear, nearest) = (wgpu::FilterMode::Linear, wgpu::FilterMode::Nearest);
    (
        sampler("image nearest", nearest, wgpu::MipmapFilterMode::Nearest),
        sampler("image smooth", linear, wgpu::MipmapFilterMode::Linear),
    )
}

/// Group 1: the viewport and shift uniform, bound by a dynamic offset.
pub(crate) fn view_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("image view"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: true,
                min_binding_size: None,
            },
            count: None,
        }],
    })
}

/// The pipeline over the texture group `layout` and the `view` group.
pub(crate) fn render_pipeline(
    device: &wgpu::Device,
    (layout, view): (&wgpu::BindGroupLayout, &wgpu::BindGroupLayout),
    shader: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("image"),
        bind_group_layouts: &[Some(layout), Some(view)],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("image"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[Some(Vertex::layout())],
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
    })
}
