//! The scene's pipeline on the scene pass's two bind group layouts,
//! `vapors_renderer::projective`'s, which the shapes drawn in the pass
//! share: the camera (group 0) and a node's slot (group 1, a dynamic
//! offset into one buffer).

use vapors_app::SURFACE_FORMAT;
use vapors_scene3d::{MATERIAL_WGSL, Material, VERTEX_BYTES};

/// The depth buffer's format.
pub(crate) const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

pub(crate) use vapors_renderer::projective::{CAMERA_BYTES, NODE_BYTES as MESH_BYTES};
use vapors_renderer::projective::{PROJECTIVE_WGSL, ProjectiveLayouts};

/// What `mount` builds once: the colour pipeline, and one for each
/// application material its `Meshes` held then, in their order.
pub(crate) struct Pipeline {
    pub(crate) render: wgpu::RenderPipeline,
    pub(crate) materials: Vec<wgpu::RenderPipeline>,
    pub(crate) layouts: ProjectiveLayouts,
}

impl Pipeline {
    pub(crate) fn new(
        device: &wgpu::Device,
        materials: &[Material],
        samples: vapors_renderer::Samples,
    ) -> Self {
        let layouts = ProjectiveLayouts::new(device);
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("scene3d"),
            bind_group_layouts: &[Some(&layouts.camera), Some(&layouts.node)],
            immediate_size: 0,
        });
        let source = [PROJECTIVE_WGSL, include_str!("shader.wgsl")].concat();
        let shader = shader(device, "scene3d", source);
        let materials = materials
            .iter()
            .map(|material| material_pipeline(device, (&layout, material), samples))
            .collect();
        Self {
            render: render_pipeline(
                device,
                (&layout, &shader),
                ("vs_main", "fs_main"),
                &vertex_buffers(),
                samples,
            ),
            materials,
            layouts,
        }
    }
}

/// The pipeline drawing meshes with `material`, at the pass's `samples`.
fn material_pipeline(
    device: &wgpu::Device,
    (layout, material): (&wgpu::PipelineLayout, &Material),
    samples: vapors_renderer::Samples,
) -> wgpu::RenderPipeline {
    let own = material.wgsl.as_str();
    let source = [
        PROJECTIVE_WGSL,
        MATERIAL_WGSL,
        own,
        include_str!("material.wgsl"),
    ]
    .concat();
    let shader = shader(device, material.label, source);
    let entries = ("vs_material", "fs_material");
    render_pipeline(
        device,
        (layout, &shader),
        entries,
        &vertex_buffers(),
        samples,
    )
}

pub(crate) fn shader(device: &wgpu::Device, label: &str, source: String) -> wgpu::ShaderModule {
    vapors_renderer::pipelines::shader_module(
        device,
        wgpu::ShaderModuleDescriptor {
            label: Some(label),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        },
    )
}

/// The vertex buffer every pipeline reads: positions and shades.
pub(crate) fn vertex_buffers() -> [Option<wgpu::VertexBufferLayout<'static>>; 1] {
    [Some(wgpu::VertexBufferLayout {
        array_stride: VERTEX_BYTES as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &VERTEX_ATTRIBUTES,
    })]
}

pub(crate) fn render_pipeline(
    device: &wgpu::Device,
    (layout, shader): (&wgpu::PipelineLayout, &wgpu::ShaderModule),
    (vertex, fragment_entry): (&str, &str),
    buffers: &[Option<wgpu::VertexBufferLayout<'_>>],
    samples: vapors_renderer::Samples,
) -> wgpu::RenderPipeline {
    vapors_renderer::pipelines::render_pipeline(
        device,
        &wgpu::RenderPipelineDescriptor {
            label: Some("scene3d"),
            layout: Some(layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some(vertex),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers,
            },
            fragment: Some(fragment(shader, fragment_entry)),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(depth_test()),
            multisample: samples.state(),
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

fn fragment<'a>(shader: &'a wgpu::ShaderModule, entry: &'a str) -> wgpu::FragmentState<'a> {
    wgpu::FragmentState {
        module: shader,
        entry_point: Some(entry),
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
