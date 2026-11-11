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
    /// Records a render pass named `label` that loads and stores the
    /// view, under the scissor, and runs `draw` in it.
    pub fn draw_pass(&self, label: &str, draw: impl FnOnce(&mut wgpu::RenderPass<'_>)) {
        self.encoder.record(|encoder| {
            let mut pass = load_pass(encoder, self.view, label);
            if let Some(s) = self.scissor {
                pass.set_scissor_rect(s.x, s.y, s.width, s.height);
            }
            draw(&mut pass);
        });
    }
}

fn load_pass<'e>(
    encoder: &'e mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    label: &str,
) -> wgpu::RenderPass<'e> {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some(label),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    })
}
