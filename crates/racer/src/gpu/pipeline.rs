//! Shared bindings and depth state for ground and solid geometry.
pub(super) fn create(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind: &wgpu::BindGroupLayout,
    shader: &wgpu::ShaderModule,
    solid: bool,
) -> wgpu::RenderPipeline {
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("racer"),
        bind_group_layouts: &[Some(bind)],
        immediate_size: 0,
    });
    let attributes = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32];
    let buffers = [Some(wgpu::VertexBufferLayout {
        array_stride: 28,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &attributes,
    })];
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(if solid {
            "racer solids"
        } else {
            "racer ground"
        }),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some(if solid { "vs_solid" } else { "vs_main" }),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: if solid { &buffers } else { &[] },
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some(if solid { "fs_solid" } else { "fs_main" }),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(solid),
            depth_compare: if solid {
                Some(wgpu::CompareFunction::LessEqual)
            } else {
                Some(wgpu::CompareFunction::Always)
            },
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}
