//! Where a draw goes, and the render pass every pipeline records into
//! the frame's encoder.

use vapors_scene::{Scissor, Size};

use crate::{DepthAttachment, FrameEncoder, Samples};

/// Where a draw goes: the target view and its size in pixels, and the
/// frame's encoder the draw is recorded into. Nothing reaches the view
/// until the encoder is submitted. A draw whose bytes need a buffer over
/// the device's `max_buffer_size` is not recorded; its refusal is kept
/// on the encoder ([`FrameEncoder::take_refusal`], [`FrameEncoder::submit`]).
#[derive(Clone, Copy)]
pub struct Target<'a> {
    /// Device the pipeline was made on.
    pub device: &'a wgpu::Device,
    /// Queue the draws' buffer and texture writes go to.
    pub queue: &'a wgpu::Queue,
    /// The frame's encoder: draws are recorded into it.
    pub encoder: &'a FrameEncoder,
    /// The view drawn into. At one sample, pixels under no quad keep
    /// what was there; at 4, each pass's resolve writes every pixel of it
    /// from `multisampled`.
    pub view: &'a wgpu::TextureView,
    /// At 4 samples, the multisampled colour view the pass draws into,
    /// resolved into `view` at the end of each pass
    /// (`docs/plans/flat-scene-f7.md`); `None` at one sample. It must be
    /// cleared in this frame's encoder (`clear_target`) before a pass on
    /// it loads: what it held before is not what `view` holds.
    pub multisampled: Option<&'a wgpu::TextureView>,
    /// The frame's depth attachment, when the frame paints a scene;
    /// `None` otherwise. It must have the pass's samples and the
    /// viewport's size ([`Target::draw_pass`] refuses it otherwise). A 2D
    /// draw takes the variant of its pipeline for it (`DepthVariants`),
    /// and neither reads nor writes it. The encoder clears it on its
    /// first pass in the encoder.
    pub depth: Option<&'a DepthAttachment>,
    /// Size of the view in pixels.
    pub viewport: Size,
    /// Where drawing may change pixels; `None` for the whole view.
    pub scissor: Option<Scissor>,
}

impl Target<'_> {
    /// The views a pass on this target attaches.
    #[must_use]
    pub fn views(&self) -> PassViews<'_> {
        PassViews {
            view: self.view,
            multisampled: self.multisampled,
            depth: self.depth.map(DepthAttachment::view),
        }
    }

    /// Runs `draw`, a draw by the pipeline `made` names (its label and
    /// the samples it was made for), in the encoder's render pass on the
    /// view, which loads and stores it, under the scissor. Consecutive
    /// draws on one view share the pass.
    ///
    /// # Panics
    ///
    /// When the pipeline's samples are not the target's (4 with a
    /// multisampled view, else 1), or the depth attachment's samples or
    /// size are not the target's: the pass would refuse it, after submit
    /// and without naming the pipeline or the attachment.
    pub fn draw_pass(&self, made: (&str, Samples), draw: impl FnOnce(&mut wgpu::RenderPass<'_>)) {
        self.assert_samples(made);
        self.assert_depth(made.0);
        self.encoder.draw(self, |pass| draw(pass));
    }

    /// The samples a pass on this target has.
    #[must_use]
    pub fn samples(&self) -> Samples {
        if self.multisampled.is_some() {
            Samples::Four
        } else {
            Samples::One
        }
    }

    fn assert_depth(&self, pipeline: &str) {
        let Some(depth) = self.depth else {
            return;
        };
        let (target, attached) = (
            (self.samples(), self.viewport),
            (depth.samples(), depth.size()),
        );
        assert!(
            attached == target,
            "{pipeline}: a depth attachment of {} samples and {:?} drawn with a target of {} \
             samples and {:?} (target.rs); make the DepthAttachment with the target's samples \
             and viewport; reproduce with `cargo xtask test`",
            attached.0.count(),
            attached.1,
            target.0.count(),
            target.1
        );
    }

    fn assert_samples(&self, (pipeline, samples): (&str, Samples)) {
        let target = self.samples();
        assert!(
            samples == target,
            "{pipeline}: a pipeline made for {} samples drawn into a target of {} (target.rs); \
             make the pipeline with the target's PassFormat; reproduce with `cargo xtask test`",
            samples.count(),
            target.count()
        );
    }
}

/// The views of a pass: the target's, and at 4 samples the multisampled
/// one drawn into and resolved into it.
#[derive(Clone, Copy)]
pub struct PassViews<'a> {
    /// The target's view: the attachment, or the resolve target.
    pub view: &'a wgpu::TextureView,
    /// The multisampled attachment, at 4 samples.
    pub multisampled: Option<&'a wgpu::TextureView>,
    /// The depth attachment, when the frame paints a scene.
    pub depth: Option<&'a wgpu::TextureView>,
}
