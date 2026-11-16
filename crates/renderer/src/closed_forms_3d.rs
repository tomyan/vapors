//! Closed-form shapes on a scene node's plane (docs/plans/decal.md
//! section 2): the page's instances (`ClosedFormShape`, their affine now
//! to the node's plane units), drawn inside a scene's render pass under
//! the camera and the node's world matrix, with the page's coverage.

use crate::projective::{Bound, ProjectiveLayouts, ScenePassFormats, ScenePipeline};
use crate::{ClosedFormShape, WorkCounter};

/// The scene pipeline's whole shader: the uniforms, the scene's vertex
/// stage and the page's coverage. Public so a test can run the shipped
/// vertex stage with a fragment of its own (`tests/support/footprint.rs`).
pub const CLOSED_FORMS_3D_WGSL: &str = concat!(
    include_str!("projective.wgsl"),
    include_str!("closed_forms_3d.wgsl"),
    include_str!("closed_forms_shared.wgsl")
);

/// Bytes per closed-form instance: five four-float attributes.
pub const CLOSED_FORM_STRIDE: u64 = 5 * 16;

/// A pipeline that draws closed-form shapes in a scene pass. Create once
/// per scene renderer, before the first frame.
pub struct ProjectiveClosedForms(wgpu::RenderPipeline);

impl ProjectiveClosedForms {
    /// Builds the pipeline on `layouts` for a pass of `formats`.
    #[must_use]
    pub fn new(
        device: &wgpu::Device,
        layouts: &ProjectiveLayouts,
        formats: ScenePassFormats,
    ) -> Self {
        let source = CLOSED_FORMS_3D_WGSL;
        let attributes = wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4, 2 => Float32x4, 3 => Float32x4, 4 => Float32x4];
        let instance = wgpu::VertexBufferLayout {
            array_stride: CLOSED_FORM_STRIDE,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &attributes,
        };
        let shader = ScenePipeline {
            label: "closed forms 3d",
            source,
            buffers: &[Some(instance)],
        };
        Self(shader.build(device, layouts, formats))
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
            bytes.is_multiple_of(CLOSED_FORM_STRIDE),
            "closed forms 3d: {bytes} instance bytes, not a whole number of {CLOSED_FORM_STRIDE}-byte instances; pass bytes() of a run (closed_forms_3d.rs)"
        );
        let count = u32::try_from(bytes / CLOSED_FORM_STRIDE).unwrap_or_else(|_| {
            panic!("closed forms 3d: {bytes} instance bytes are more instances than fit one draw")
        });
        pass.set_pipeline(&self.0);
        pass.set_bind_group(0, bound.camera, &[]);
        pass.set_bind_group(1, bound.node, &[bound.offset]);
        pass.set_vertex_buffer(0, instances);
        work.draw(pass, 0..6, 0..count);
    }
}
