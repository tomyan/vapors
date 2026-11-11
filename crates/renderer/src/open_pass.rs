//! The render pass a [`FrameEncoder`](crate::FrameEncoder) keeps open
//! on one view, so consecutive draws there share it. On ANV each render
//! pass cost about 110 µs of GPU time, and a frame of 9017 passes
//! failed `CommandEncoder::finish` with Out of Memory (vapors #181).

/// A render pass on `view`, open until it is dropped. While it is open
/// its encoder refuses every other command.
pub(crate) struct OpenPass {
    view: wgpu::TextureView,
    size: (u32, u32),
    pass: wgpu::RenderPass<'static>,
}

impl OpenPass {
    /// Begins a pass on `view` that first loads it as `load` says.
    pub(crate) fn begin(
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        load: wgpu::LoadOp<wgpu::Color>,
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
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        let texture = view.texture();
        Self {
            view: view.clone(),
            size: (texture.width(), texture.height()),
            pass: pass.forget_lifetime(),
        }
    }

    /// Whether this pass draws into `view`.
    pub(crate) fn is_on(&self, view: &wgpu::TextureView) -> bool {
        self.view == *view
    }

    /// The pass, its scissor set to `scissor` or, for `None`, the whole
    /// view: a scissor set by an earlier draw in the pass would
    /// otherwise still apply. The whole view is the texture's first mip
    /// level; the frame's views are whole textures.
    pub(crate) fn scissored(
        &mut self,
        scissor: Option<vapors_scene::Scissor>,
    ) -> &mut wgpu::RenderPass<'static> {
        match scissor {
            Some(s) => self.pass.set_scissor_rect(s.x, s.y, s.width, s.height),
            None => self.pass.set_scissor_rect(0, 0, self.size.0, self.size.1),
        }
        &mut self.pass
    }
}
