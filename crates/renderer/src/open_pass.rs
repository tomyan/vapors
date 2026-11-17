//! The render pass a [`FrameEncoder`](crate::FrameEncoder) keeps open
//! on one view, so consecutive draws there share it. On ANV each render
//! pass cost about 110 µs of GPU time, and a frame of 9017 passes
//! failed `CommandEncoder::finish` with Out of Memory (vapors #181).

use vapors_scene::{Scissor, Size};

use crate::target::PassViews;

/// A render pass on `view`, open until it is dropped. While it is open
/// its encoder refuses every other command.
pub(crate) struct OpenPass {
    view: wgpu::TextureView,
    /// The multisampled view drawn into, at 4 samples.
    multisampled: Option<wgpu::TextureView>,
    /// The depth attachment, when the frame paints a scene.
    depth: Option<wgpu::TextureView>,
    pass: wgpu::RenderPass<'static>,
}

impl OpenPass {
    /// Begins a pass on `views` that first loads its colour as `load`
    /// and its depth, when it has one, as `depth` says, and writes
    /// `timestamps` at its start and end. At 4 samples
    /// the attachment is the multisampled view, stored (a later pass on
    /// the view in the frame loads it) and resolved into the target's
    /// view at the pass's end (`docs/plans/flat-scene-f7.md`, Q3).
    pub(crate) fn begin(
        encoder: &mut wgpu::CommandEncoder,
        views: PassViews<'_>,
        (load, depth): (wgpu::LoadOp<wgpu::Color>, wgpu::LoadOp<f32>),
        timestamps: Option<wgpu::RenderPassTimestampWrites<'_>>,
    ) -> Self {
        let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("frame"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: views.multisampled.unwrap_or(views.view),
                depth_slice: None,
                resolve_target: views.multisampled.map(|_| views.view),
                ops: wgpu::Operations {
                    load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: views.depth.map(|view| depth_attachment(view, depth)),
            timestamp_writes: timestamps,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        Self {
            view: views.view.clone(),
            multisampled: views.multisampled.cloned(),
            depth: views.depth.cloned(),
            pass: pass.forget_lifetime(),
        }
    }

    /// Whether this pass draws into `views`: the same target view, and
    /// the same multisampled view or none.
    pub(crate) fn is_on(&self, views: PassViews<'_>) -> bool {
        self.view == *views.view
            && self.multisampled.as_ref() == views.multisampled
            && self.depth.as_ref() == views.depth
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

/// The depth attachment `view`, loaded as `load` (the encoder clears it
/// on its first pass, `FrameEncoder::depth_load`) and stored for a later
/// pass. F8d decides when it may be discarded.
fn depth_attachment(
    view: &wgpu::TextureView,
    load: wgpu::LoadOp<f32>,
) -> wgpu::RenderPassDepthStencilAttachment<'_> {
    wgpu::RenderPassDepthStencilAttachment {
        view,
        depth_ops: Some(wgpu::Operations {
            load,
            store: wgpu::StoreOp::Store,
        }),
        stencil_ops: None,
    }
}
