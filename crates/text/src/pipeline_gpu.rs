//! The wgpu objects behind [`TextPipeline`](crate::TextPipeline): the
//! atlas texture, the bind group layout and the render pipeline.

use crate::pipeline::VERTEX_SIZE;

pub(crate) fn coverage_texture(device: &wgpu::Device, size: u32) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("glyph atlas coverage"),
        size: wgpu::Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    })
}

pub(crate) fn bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("text"),
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
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: None,
                },
                count: None,
            },
            // the transform table (vapors-renderer transform_table.rs): the
            // vertex places by it, the fragment's clip walk reads each
            // clip's frame in it
            storage(3, wgpu::ShaderStages::VERTEX_FRAGMENT),
            // the clip table (vapors-renderer clip_table.rs, clip.wgsl)
            storage(4, wgpu::ShaderStages::FRAGMENT),
        ],
    })
}

/// A read-only storage buffer at `binding`, seen by `visibility`.
fn storage(binding: u32, visibility: wgpu::ShaderStages) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

pub(crate) fn render_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    (pass, depth): (vapors_renderer::PassFormat, vapors_renderer::Depth),
) -> wgpu::RenderPipeline {
    let targets = [Some(color_target(pass.format))];
    vapors_renderer::pipelines::render_pipeline(
        device,
        &wgpu::RenderPipelineDescriptor {
            label: Some("text"),
            layout: Some(layout),
            vertex: vertex_state(shader),
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &targets,
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: vapors_renderer::depth_2d(depth),
            multisample: pass.samples.state(),
            multiview_mask: None,
            cache: None,
        },
    )
}

const ATTRIBUTES: [wgpu::VertexAttribute; 6] = wgpu::vertex_attr_array![
    0 => Float32x2, 1 => Float32x2, 2 => Float32x4, 3 => Uint32, 4 => Uint32, 5 => Uint32
];
const BUFFERS: [Option<wgpu::VertexBufferLayout<'static>>; 1] = [Some(wgpu::VertexBufferLayout {
    array_stride: VERTEX_SIZE,
    step_mode: wgpu::VertexStepMode::Vertex,
    attributes: &ATTRIBUTES,
})];

fn vertex_state(shader: &wgpu::ShaderModule) -> wgpu::VertexState<'_> {
    wgpu::VertexState {
        module: shader,
        entry_point: Some("vs_main"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        buffers: &BUFFERS,
    }
}

fn color_target(format: wgpu::TextureFormat) -> wgpu::ColorTargetState {
    wgpu::ColorTargetState {
        format,
        blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
        write_mask: wgpu::ColorWrites::ALL,
    }
}
