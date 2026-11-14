//! Drawing a run of mesh items (docs/plans/one-scene.md 1.2): each the
//! triangles of an entry in [`MeshEntries`] under its own transform and
//! colour, one draw per item, in order. The vertex shader moves each
//! outline vertex out by the item's extrusion along its miter offset
//! and takes the move off its edge distances, so the ramp straddles the
//! outline; the fragment shader takes coverage from the nearest outline
//! edge's distance in device pixels. Instance bytes are kept between
//! frames as a quad run's are; the entries' vertices are the store's.

use crate::instanced::{float_bytes, viewport_bytes};
use crate::kept_bind_group::KeptBindGroup;
use crate::mesh_entries::{MeshEntries, MeshEntry};
use crate::uniform_slots::UniformSlots;
use crate::{GpuWork, KeptRange, MeshShape, Retained, Shift, Target, WorkCounter};

/// Bytes per instance: three four-float attributes.
const INSTANCE_BYTES: u64 = 48;

/// A pipeline that draws mesh items. Create once per target format.
pub struct MeshPipeline {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    viewports: UniformSlots,
    bind_group: KeptBindGroup,
    kept: Retained,
    work: WorkCounter,
}

impl MeshPipeline {
    /// Build the pipeline for targets of `format`.
    #[must_use]
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let layout = crate::quads_setup::bind_group_layout(device);
        Self {
            pipeline: crate::meshes_setup::render_pipeline(device, &layout, format),
            layout,
            viewports: UniformSlots::new(device, "meshes"),
            bind_group: KeptBindGroup::NONE,
            kept: Retained::new(device, "meshes", wgpu::BufferUsages::VERTEX),
            work: WorkCounter::default(),
        }
    }

    /// The instance bytes of `shapes`, as a draw keeps them.
    #[must_use]
    pub fn bytes(shapes: &[MeshShape]) -> Vec<u8> {
        float_bytes(shapes.iter().flat_map(MeshShape::instance))
    }

    /// Draws `shapes`, each of the entry beside it in `drawn`, without
    /// keeping their bytes past the frame: for tests and one-off draws.
    ///
    /// # Panics
    /// As [`MeshPipeline::draw_kept`].
    pub fn draw(
        &mut self,
        target: &Target<'_>,
        entries: &mut MeshEntries,
        items: &[(MeshShape, &MeshEntry)],
    ) {
        let shapes: Vec<MeshShape> = items.iter().map(|(shape, _)| *shape).collect();
        let Some(kept) = self.keep(target, &Self::bytes(&shapes)) else {
            return;
        };
        let drawn: Vec<&MeshEntry> = items.iter().map(|(_, entry)| *entry).collect();
        self.draw_kept(target, &kept, Shift::NONE, (entries, &drawn));
        self.release(kept);
    }

    /// Writes `bytes` ([`MeshPipeline::bytes`] of a run) into the kept
    /// buffer; `None` when refused (on the target's encoder).
    pub fn keep(&mut self, target: &Target<'_>, bytes: &[u8]) -> Option<KeptRange> {
        self.kept
            .keep(target.encoder, target.queue, &self.work, bytes)
    }

    /// Draws the items `kept` holds, the `i`th of `drawn[i]`'s entry,
    /// moved by `shift`, one draw each.
    ///
    /// # Panics
    /// If `drawn` does not give one entry per kept instance, or `kept`
    /// or an entry is another buffer's.
    pub fn draw_kept(
        &mut self,
        target: &Target<'_>,
        kept: &KeptRange,
        shift: Shift,
        (entries, drawn): (&mut MeshEntries, &[&MeshEntry]),
    ) {
        let count = self.kept.len(kept) / INSTANCE_BYTES;
        assert_eq!(
            count,
            drawn.len() as u64,
            "meshes: {count} kept instances and {} entries; give each instance its entry",
            drawn.len()
        );
        let Some(viewport) = self.viewport_slot(target, shift) else {
            return;
        };
        let Self {
            pipeline,
            layout,
            viewports,
            bind_group,
            kept: instances_buffer,
            work,
        } = self;
        let bind_group = bind_group.of(viewports.generation(), || {
            made_bind_group(target, layout, work, viewports)
        });
        let instances = instances_buffer.slice(target.encoder, kept);
        let vertices = entries.slices(target, drawn);
        let (pipeline, work) = (&*pipeline, &*work);
        target.draw_pass(|pass| {
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, bind_group, &[viewport]);
            pass.set_vertex_buffer(1, instances);
            for (at, (slice, entry)) in (0u32..).zip(vertices.into_iter().zip(drawn)) {
                pass.set_vertex_buffer(0, slice);
                work.draw(pass, 0..entry.vertices, at..at + 1);
            }
        });
    }

    /// Gives `kept`'s bytes back.
    pub fn release(&mut self, kept: KeptRange) {
        self.kept.release(kept);
    }

    /// Bytes held in the kept buffer.
    #[must_use]
    pub fn retained_bytes(&self) -> u64 {
        self.kept.live_bytes()
    }

    /// The bind groups kept: one once this pipeline has drawn.
    #[must_use]
    pub fn bind_groups_kept(&self) -> u64 {
        self.bind_group.kept()
    }

    /// The work its draws issued since the last take; the count restarts.
    pub fn take_work(&self) -> GpuWork {
        self.work.take()
    }

    fn viewport_slot(&mut self, target: &Target<'_>, shift: Shift) -> Option<u32> {
        let viewport = viewport_bytes(target.viewport, shift);
        self.viewports
            .slot(target.encoder, target.queue, &self.work, viewport)
    }
}

fn made_bind_group(
    target: &Target<'_>,
    layout: &wgpu::BindGroupLayout,
    work: &WorkCounter,
    viewports: &UniformSlots,
) -> wgpu::BindGroup {
    work.create_bind_group(
        target.device,
        &wgpu::BindGroupDescriptor {
            label: Some("meshes"),
            layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: viewports.binding(),
            }],
        },
    )
}
