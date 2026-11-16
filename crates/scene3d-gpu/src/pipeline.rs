//! The scene's pipeline on the scene pass's two bind group layouts,
//! `vapors_renderer::projective`'s, which the shapes drawn in the pass
//! share: the camera (group 0) and a node's slot (group 1, a dynamic
//! offset into one buffer).

use vapors_app::SURFACE_FORMAT;
use vapors_scene3d::VERTEX_BYTES;

/// The depth buffer's format.
pub(crate) const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

pub(crate) use vapors_renderer::projective::{CAMERA_BYTES, NODE_BYTES as MESH_BYTES};
use vapors_renderer::projective::{PROJECTIVE_WGSL, ProjectiveLayouts};

/// What `mount` builds once.
pub(crate) struct Pipeline {
    pub(crate) render: wgpu::RenderPipeline,
    pub(crate) layouts: ProjectiveLayouts,
}

impl Pipeline {
    pub(crate) fn new(device: &wgpu::Device) -> Self {
        let layouts = ProjectiveLayouts::new(device);
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("scene3d"),
            bind_group_layouts: &[Some(&layouts.camera), Some(&layouts.node)],
            immediate_size: 0,
        });
        let source = [PROJECTIVE_WGSL, include_str!("shader.wgsl")].concat();
        let shader = vapors_renderer::pipelines::shader_module(
            device,
            wgpu::ShaderModuleDescriptor {
                label: Some("scene3d"),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            },
        );
        Self {
            render: render_pipeline(device, &layout, &shader),
            layouts,
        }
    }
}

fn render_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
) -> wgpu::RenderPipeline {
    let buffers = [Some(wgpu::VertexBufferLayout {
        array_stride: VERTEX_BYTES as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &VERTEX_ATTRIBUTES,
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

/// A vertex: its position, then its shade (`geometry.rs`,
/// `vertex_bytes`); `pipeline_tests.rs` holds them to [`VERTEX_BYTES`].
const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 2] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32];

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

#[cfg(test)]
#[path = "pipeline_tests.rs"]
mod tests;
