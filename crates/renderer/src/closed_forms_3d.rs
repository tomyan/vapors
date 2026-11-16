//! Closed-form shapes on a scene node's plane (docs/plans/decal.md
//! section 2): the page's instances (`ClosedFormShape`, their affine now
//! to the node's plane units), drawn inside a scene's render pass under
//! the camera and the node's world matrix, with the page's coverage.

use crate::projective::{ProjectiveLayouts, SHAPE_BLEND, shape_depth};
use crate::{ClosedFormShape, WorkCounter};

/// Bytes per instance: five four-float attributes.
const STRIDE: u64 = 5 * 16;

/// A pipeline that draws closed-form shapes in a scene pass. Create once
/// per scene renderer, before the first frame.
pub struct ProjectiveClosedForms(wgpu::RenderPipeline);

/// What the scene's pass targets.
#[derive(Clone, Copy, Debug)]
pub struct ScenePassFormats {
    /// The colour attachment's format.
    pub colour: wgpu::TextureFormat,
    /// The depth attachment's format.
    pub depth: wgpu::TextureFormat,
    /// The depth bias shapes draw with.
    pub bias: wgpu::DepthBiasState,
}

impl ProjectiveClosedForms {
    /// Builds the pipeline on `layouts` for a pass of `formats`.
    #[must_use]
    pub fn new(
        device: &wgpu::Device,
        layouts: &ProjectiveLayouts,
        formats: ScenePassFormats,
    ) -> Self {
        let source = concat!(
            include_str!("closed_forms_3d.wgsl"),
            include_str!("closed_forms_shared.wgsl")
        );
        let shader = crate::pipelines::shader_module(
            device,
            wgpu::ShaderModuleDescriptor {
                label: Some("closed forms 3d"),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            },
        );
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("closed forms 3d"),
            bind_group_layouts: &[Some(&layouts.camera), Some(&layouts.node)],
            immediate_size: 0,
        });
        Self(render_pipeline(device, &layout, &shader, formats))
    }

    /// The instance bytes of `shapes`, as [`ProjectiveClosedForms::record`]
    /// reads them: the page's.
    #[must_use]
    pub fn bytes(shapes: &[ClosedFormShape]) -> Vec<u8> {
        crate::ClosedFormPipeline::bytes(shapes)
    }

    /// Draws the instances in `instances` (bytes of
    /// [`ProjectiveClosedForms::bytes`]), later over earlier, in one draw
    /// in `pass` under the camera and the node slot at `offset`.
    ///
    /// # Panics
    ///
    /// If `instances` is not a whole number of instances, or holds more
    /// than one draw can count.
    pub fn record(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        bound: Bound<'_>,
        instances: wgpu::BufferSlice<'_>,
        work: &WorkCounter,
    ) {
        let bytes = instances.size();
        assert!(
            bytes.is_multiple_of(STRIDE),
            "closed forms 3d: {bytes} instance bytes, not a whole number of {STRIDE}-byte instances; pass bytes() of a run (closed_forms_3d.rs)"
        );
        let count = u32::try_from(bytes / STRIDE).unwrap_or_else(|_| {
            panic!("closed forms 3d: {bytes} instance bytes are more instances than fit one draw")
        });
        pass.set_pipeline(&self.0);
        pass.set_bind_group(0, bound.camera, &[]);
        pass.set_bind_group(1, bound.node, &[bound.offset]);
        pass.set_vertex_buffer(0, instances);
        work.draw(pass, 0..6, 0..count);
    }
}

/// The camera and node a draw is under.
#[derive(Clone, Copy, Debug)]
pub struct Bound<'a> {
    /// Group 0.
    pub camera: &'a wgpu::BindGroup,
    /// Group 1.
    pub node: &'a wgpu::BindGroup,
    /// The node's slot in group 1's buffer.
    pub offset: u32,
}

fn render_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    formats: ScenePassFormats,
) -> wgpu::RenderPipeline {
    let attributes = wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4, 2 => Float32x4, 3 => Float32x4, 4 => Float32x4];
    let instance = wgpu::VertexBufferLayout {
        array_stride: STRIDE,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &attributes,
    };
    crate::pipelines::render_pipeline(
        device,
        &wgpu::RenderPipelineDescriptor {
            label: Some("closed forms 3d"),
            layout: Some(layout),
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
                    format: formats.colour,
                    blend: Some(SHAPE_BLEND),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(shape_depth(formats.depth, formats.bias)),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        },
    )
}
