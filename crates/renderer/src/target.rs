//! Where a draw goes, and the render pass every pipeline records into
//! the frame's encoder.

use vapors_scene::{Scissor, Size};

use crate::FrameEncoder;

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
    /// The view drawn into; pixels under no quad keep what was there.
    pub view: &'a wgpu::TextureView,
    /// Size of the view in pixels.
    pub viewport: Size,
    /// Where drawing may change pixels; `None` for the whole view.
    pub scissor: Option<Scissor>,
}

impl Target<'_> {
    /// Runs `draw` in the encoder's render pass on the view, which loads
    /// and stores it, under the scissor. Consecutive draws on one view
    /// share the pass.
    pub fn draw_pass(&self, draw: impl FnOnce(&mut wgpu::RenderPass<'_>)) {
        self.encoder
            .draw(self.view, self.scissor, |pass| draw(pass));
    }
}
