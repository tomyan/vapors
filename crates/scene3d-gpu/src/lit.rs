//! The lit pipeline (`docs/plans/lighting.md`, LT3): made by the first
//! frame that draws a lit mesh, RELAXED against building every pipeline
//! at mount; the parity plan's M3 retires it. Also the lit refusals.

use crate::FrameError;
use crate::draw::MeshNode;
use crate::pipeline::{Pipeline, render_pipeline, shader, vertex_buffers};
use vapors_renderer::projective::PROJECTIVE_WGSL;
use vapors_scene3d::{MaterialId, NORMAL_BYTES, NodeId, Scene};

/// The pipelines (Lambert; Lambert and specular), and the group 2 they
/// read: the lights uniform.
pub(crate) struct LitPipeline {
    pub(crate) pipeline: wgpu::RenderPipeline,
    pub(crate) glossy: wgpu::RenderPipeline,
    pub(crate) bind_group: wgpu::BindGroup,
}

impl LitPipeline {
    pub(crate) fn new(
        device: &wgpu::Device,
        (pipeline, lights): (&Pipeline, &wgpu::Buffer),
        samples: vapors_renderer::Samples,
    ) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("scene3d lit"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(crate::lights::LIGHTS_BYTES),
                },
                count: None,
            }],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("scene3d lit"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: lights.as_entire_binding(),
            }],
        });
        let layouts = [
            Some(&pipeline.layouts.camera),
            Some(&pipeline.layouts.node),
            Some(&layout),
        ];
        let [pipeline, glossy] = lit_pipelines(device, &layouts, samples);
        Self {
            pipeline,
            glossy,
            bind_group,
        }
    }
}

fn lit_pipelines(
    device: &wgpu::Device,
    layouts: &[Option<&wgpu::BindGroupLayout>],
    samples: vapors_renderer::Samples,
) -> [wgpu::RenderPipeline; 2] {
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("scene3d lit"),
        bind_group_layouts: layouts,
        immediate_size: 0,
    });
    let source = [
        PROJECTIVE_WGSL,
        include_str!("lighting.wgsl"),
        include_str!("lit.wgsl"),
    ];
    let module = shader(device, "scene3d lit", source.concat());
    let [positions] = vertex_buffers();
    let normals = wgpu::VertexBufferLayout {
        array_stride: NORMAL_BYTES as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![2 => Float32x3],
    };
    let buffers = [positions, Some(normals)];
    ["fs_lit", "fs_glossy"].map(|fragment| {
        let entries = ("vs_lit", fragment);
        render_pipeline(device, (&layout, &module), entries, &buffers, samples)
    })
}

/// Refuses lit meshes with no normals or an application material, or
/// with no light and zero ambient.
pub(crate) fn refuse(scene: &Scene, nodes: &[MeshNode]) -> Result<(), FrameError> {
    let lit: Vec<&MeshNode> = nodes.iter().filter(|mesh| mesh.lit).collect();
    let Some(first) = lit.first().map(|mesh| mesh.node) else {
        return Ok(());
    };
    if let Some(mesh) = lit.iter().find(|m| m.handle.geometry().normals().is_none()) {
        let (node, geometry) = (mesh.node, mesh.geometry);
        return Err(FrameError::LitWithoutNormals { node, geometry });
    }
    if let Some(&(node, material)) = lit_materials(&lit).first() {
        return Err(FrameError::LitWithMaterial { node, material });
    }
    let dark = scene.ambient().linear().iter().all(|&c| c <= 0.0);
    if dark && scene.lights().is_empty() {
        return Err(FrameError::LitWithoutLight(first));
    }
    Ok(())
}

/// Each lit mesh that also names an application material, which the lit
/// pipeline would silently ignore.
fn lit_materials(lit: &[&MeshNode]) -> Vec<(NodeId, MaterialId)> {
    let named = |mesh: &&MeshNode| mesh.material.map(|material| (mesh.node, material));
    lit.iter().filter_map(named).collect()
}
