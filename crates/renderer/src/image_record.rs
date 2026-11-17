//! Recording one of the image pipeline's draws: the quad's vertices,
//! this frame's or kept, under the texture's bind group and the view
//! group: the view slot (the viewport, the quad's frame entry and its
//! row's entry) by its dynamic offset, and the transform table.

use std::ops::Range;

use super::ImagePipeline;
use crate::frame_arena::Placed;
use crate::image_binding::TextureBinding;
use crate::kept_bind_group::KeptBindGroup;
use crate::uniform_slots::UniformSlots;
use crate::{KeptRange, Target, TransformTable, WorkCounter};

/// Where a draw's vertices are: this frame's arena, or kept.
#[derive(Clone, Copy)]
pub(super) enum Quads<'a> {
    Placed(&'a Placed),
    Kept(&'a KeptRange),
}

impl ImagePipeline {
    /// The dynamic offset of the target's viewport and the quad's
    /// `frame` and `row` entries in the view slots; `None` when refused
    /// (the refusal is on the encoder).
    pub(super) fn view_slot(
        &mut self,
        target: &Target<'_>,
        (frame, row): (u32, u32),
    ) -> Option<u32> {
        let value = view_bytes(target.viewport, (frame, row));
        let (encoder, queue) = (target.encoder, target.queue);
        self.views.slot(encoder, queue, &self.work, value)
    }

    /// Records a draw of `vertices` of `from` with `bound`'s texture and
    /// the view slot at `view`, through the view group over `table` (the
    /// pipeline's identity table for `None`).
    pub(super) fn record(
        &mut self,
        target: &Target<'_>,
        bound: &TextureBinding,
        (from, vertices): (Quads<'_>, Range<u32>),
        (table, view): (Option<&TransformTable>, u32),
    ) {
        let _encoding = tracing::trace_span!("draw step", phase = "encode").entered();
        let slice = match from {
            Quads::Placed(placed) => placed.slice(),
            Quads::Kept(kept) => self.kept.slice(target.encoder, kept),
        };
        let (kept_group, table) = match table {
            Some(table) => (&mut self.table_group, table),
            None => (&mut self.own_group, &self.identity),
        };
        let views = &self.views;
        let view_group = view_group(
            kept_group,
            (&self.view_layout, views, table),
            (target, &self.work),
        );
        target.draw_pass(("images", self.samples), |pass| {
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, bound.bind_group(), &[]);
            pass.set_bind_group(1, view_group, &[view]);
            pass.set_vertex_buffer(0, slice);
            self.work.draw(pass, vertices, 0..1);
        });
    }
}

/// The view uniform's bytes: the viewport's size, two floats, then the
/// quad's frame's entry and its row's.
#[allow(clippy::cast_precision_loss)] // viewports are far below 2^24 pixels
fn view_bytes(viewport: vapors_scene::Size, (frame, row): (u32, u32)) -> [u8; 16] {
    let mut bytes = [0; 16];
    bytes[..4].copy_from_slice(&(viewport.width as f32).to_le_bytes());
    bytes[4..8].copy_from_slice(&(viewport.height as f32).to_le_bytes());
    bytes[8..12].copy_from_slice(&frame.to_le_bytes());
    bytes[12..].copy_from_slice(&row.to_le_bytes());
    bytes
}

/// The kept view bind group over `views`' buffer and `table`, made (and
/// counted in `work`) when none is kept for their generations.
fn view_group<'a>(
    kept: &'a mut KeptBindGroup,
    (layout, views, table): (&wgpu::BindGroupLayout, &UniformSlots, &TransformTable),
    (target, work): (&Target<'_>, &WorkCounter),
) -> &'a wgpu::BindGroup {
    kept.of((views.generation(), table.generation()), || {
        let entries = [
            wgpu::BindGroupEntry {
                binding: 0,
                resource: views.binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: table.binding(),
            },
        ];
        let label = Some("image view");
        let descriptor = wgpu::BindGroupDescriptor {
            label,
            layout,
            entries: &entries,
        };
        work.create_bind_group(target.device, &descriptor)
    })
}
