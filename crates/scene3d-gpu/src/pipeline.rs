//! The scene's pipeline and its two bind group layouts: the camera's
//! view-projection (group 0) and a mesh's slot (group 1, a dynamic
//! offset into one buffer).

use vapors_app::SURFACE_FORMAT;

/// The depth buffer's format.
pub(crate) const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// Bytes in the camera uniform: one 4x4 `f32` matrix.
pub(crate) const CAMERA_BYTES: u64 = 64;

/// Bytes in a mesh uniform: its world matrix and its colour.
pub(crate) const MESH_BYTES: u64 = 80;

/// What `mount` builds once.
pub(crate) struct Pipeline {
    pub(crate) render: wgpu::RenderPipeline,
    pub(crate) camera_layout: wgpu::BindGroupLayout,
    pub(crate) mesh_layout: wgpu::BindGroupLayout,
}

impl Pipeline {
    pub(crate) fn new(device: &wgpu::Device) -> Self {
        let camera_layout = uniform_layout(device, "scene3d camera", false, CAMERA_BYTES);
        let mesh_layout = uniform_layout(device, "scene3d mesh", true, MESH_BYTES);
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("scene3d"),
            bind_group_layouts: &[Some(&camera_layout), Some(&mesh_layout)],
            immediate_size: 0,
        });
        let shader =
            vapors_renderer::pipelines::shader_module(device, wgpu::include_wgsl!("shader.wgsl"));
        Self {
            render: render_pipeline(device, &layout, &shader),
            camera_layout,
            mesh_layout,
        }
    }
}

fn uniform_layout(
    device: &wgpu::Device,
    label: &str,
    has_dynamic_offset: bool,
    bytes: u64,
) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some(label),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset,
                min_binding_size: wgpu::BufferSize::new(bytes),
            },
            count: None,
        }],
    })
}

fn render_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
) -> wgpu::RenderPipeline {
    // a vertex: its position, then its shade (`geometry.rs`, `upload`)
    let attributes = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32];
    let buffers = [Some(wgpu::VertexBufferLayout {
        array_stride: 16,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &attributes,
    })];
    vapors_renderer::pipelines::render_pipeline(
        device,
        &wgpu::RenderPipelineDescriptor {
            label: Some("scene3d"),
            layout: Some(layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &buffers,
            },
            fragment: Some(fragment(shader)),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(depth_test()),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        },
    )
}

const TARGETS: [Option<wgpu::ColorTargetState>; 1] = [Some(wgpu::ColorTargetState {
    format: SURFACE_FORMAT,
    blend: None,
    write_mask: wgpu::ColorWrites::ALL,
})];

fn fragment(shader: &wgpu::ShaderModule) -> wgpu::FragmentState<'_> {
    wgpu::FragmentState {
        module: shader,
        entry_point: Some("fs_main"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        targets: &TARGETS,
    }
}

/// Nearer fragments win: depth runs 0 at near to 1 at far.
fn depth_test() -> wgpu::DepthStencilState {
    wgpu::DepthStencilState {
        format: DEPTH_FORMAT,
        depth_write_enabled: Some(true),
        depth_compare: Some(wgpu::CompareFunction::Less),
        stencil: wgpu::StencilState::default(),
        bias: wgpu::DepthBiasState::default(),
    }
}
