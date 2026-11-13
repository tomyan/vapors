//! Recording one of the image pipeline's draws: the quad's vertices,
//! this frame's or kept, under the texture's bind group and the view
//! slot (the viewport and a scroll frame's shift) by its dynamic offset.

use std::ops::Range;

use super::ImagePipeline;
use crate::frame_arena::Placed;
use crate::image_binding::TextureBinding;
use crate::instanced::viewport_bytes;
use crate::kept_bind_group::KeptBindGroup;
use crate::uniform_slots::UniformSlots;
use crate::{KeptRange, Shift, Target, WorkCounter};

/// Where a draw's vertices are: this frame's arena, or kept.
#[derive(Clone, Copy)]
pub(super) enum Quads<'a> {
    Placed(&'a Placed),
    Kept(&'a KeptRange),
}

impl ImagePipeline {
    /// The dynamic offset of the target's viewport and `shift` in the
    /// view slots; `None` when refused (the refusal is on the encoder).
    pub(super) fn view_slot(&mut self, target: &Target<'_>, shift: Shift) -> Option<u32> {
        let value = viewport_bytes(target.viewport, shift);
        let (frame, queue) = (target.encoder, target.queue);
        self.views.slot(frame, queue, &self.work, value)
    }

    /// Records a draw of `vertices` of `from` with `bound`'s texture and
    /// the view slot at `view`, through the kept view bind group.
    pub(super) fn record(
        &mut self,
        target: &Target<'_>,
        bound: &TextureBinding,
        (from, vertices): (Quads<'_>, Range<u32>),
        view: u32,
    ) {
        let _encoding = tracing::trace_span!("draw step", phase = "encode").entered();
        let view_group = view_group(
            &mut self.view_group,
            (&self.view_layout, &self.views),
            (target, &self.work),
        );
        let slice = match from {
            Quads::Placed(placed) => placed.slice(),
            Quads::Kept(kept) => self.kept.slice(target.encoder, kept),
        };
        target.draw_pass(|pass| {
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, bound.bind_group(), &[]);
            pass.set_bind_group(1, view_group, &[view]);
            pass.set_vertex_buffer(0, slice);
            self.work.draw(pass, vertices, 0..1);
        });
    }
}

/// The kept view bind group over `views`' buffer, made (and counted in
/// `work`) for its generation when none is kept for it.
fn view_group<'a>(
    kept: &'a mut KeptBindGroup,
    (layout, views): (&wgpu::BindGroupLayout, &UniformSlots),
    (target, work): (&Target<'_>, &WorkCounter),
) -> &'a wgpu::BindGroup {
    kept.of(views.generation(), || {
        let entries = [wgpu::BindGroupEntry {
            binding: 0,
            resource: views.binding(),
        }];
        let label = Some("image view");
        let descriptor = wgpu::BindGroupDescriptor {
            label,
            layout,
            entries: &entries,
        };
        work.create_bind_group(target.device, &descriptor)
    })
}
