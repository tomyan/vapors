//! The render pass a [`FrameEncoder`](crate::FrameEncoder) keeps open
//! on one view, so consecutive draws there share it. On ANV each render
//! pass cost about 110 µs of GPU time, and a frame of 9017 passes
//! failed `CommandEncoder::finish` with Out of Memory (vapors #181).

use vapors_scene::{Scissor, Size};

/// A render pass on `view`, open until it is dropped. While it is open
/// its encoder refuses every other command.
pub(crate) struct OpenPass {
    view: wgpu::TextureView,
    pass: wgpu::RenderPass<'static>,
}

impl OpenPass {
    /// Begins a pass on `view` that first loads it as `load` says, and
    /// writes `timestamps` at its start and end.
    pub(crate) fn begin(
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        load: wgpu::LoadOp<wgpu::Color>,
        timestamps: Option<wgpu::RenderPassTimestampWrites<'_>>,
    ) -> Self {
        let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("frame"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: timestamps,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        Self {
            view: view.clone(),
            pass: pass.forget_lifetime(),
        }
    }

    /// Whether this pass draws into `view`.
    pub(crate) fn is_on(&self, view: &wgpu::TextureView) -> bool {
        self.view == *view
    }

    /// The pass, its scissor set to `scissor` or, for `None`, the whole
    /// `viewport` (the target's size, which for a view of a smaller mip
    /// level is not the texture's): a scissor set by an earlier draw in
    /// the pass would otherwise still apply.
    pub(crate) fn scissored(
        &mut self,
        scissor: Option<Scissor>,
        viewport: Size,
    ) -> &mut wgpu::RenderPass<'static> {
        match scissor {
            Some(s) => self.pass.set_scissor_rect(s.x, s.y, s.width, s.height),
            None => self
                .pass
                .set_scissor_rect(0, 0, viewport.width, viewport.height),
        }
        &mut self.pass
    }
}
