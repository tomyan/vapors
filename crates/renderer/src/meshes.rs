//! Drawing a run of mesh items (docs/plans/one-scene.md 1.2): each the
//! triangles of an entry in [`MeshEntries`] under its own transform and
//! colour, one draw per item, in order, one instance per triangle. The
//! vertex shader moves each outline corner half a device pixel out
//! across both lines of its wedge, so the ramp straddles the outline;
//! the fragment shader takes coverage from the triangle's corners'
//! wedges in device pixels. Instance bytes are kept between frames as a
//! quad run's are; the entries' triangles are the store's.

use crate::mesh_entries::{MeshEntries, MeshEntry};
use crate::meshes_bytes::INSTANCE_BYTES;
use crate::table_binding::TableBinding;
use crate::{GpuWork, KeptRange, MeshShape, Retained, Target, TransformTable, WorkCounter};

/// A pipeline that draws mesh items. Create once per target format.
pub struct MeshPipeline {
    pipeline: wgpu::RenderPipeline,
    binding: TableBinding,
    kept: Retained,
    work: WorkCounter,
}

impl MeshPipeline {
    /// Build the pipeline for targets of `format`.
    #[must_use]
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let binding = TableBinding::new(device, "meshes");
        Self {
            pipeline: crate::meshes_setup::render_pipeline(device, binding.layout(), format),
            binding,
            kept: Retained::new(device, "meshes", wgpu::BufferUsages::VERTEX),
            work: WorkCounter::default(),
        }
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
        self.binding.prepare_identity(target.queue, &self.work);
        self.draw_from(target, &kept, (None, 0), (entries, &drawn));
        self.release(kept);
    }

    /// Writes `bytes` ([`MeshPipeline::bytes`] of a run) into the kept
    /// buffer; `None` when refused (on the target's encoder).
    pub fn keep(&mut self, target: &Target<'_>, bytes: &[u8]) -> Option<KeptRange> {
        self.kept
            .keep(target.encoder, target.queue, &self.work, bytes)
    }

    /// Draws the items `kept` holds, each of the entry beside it in
    /// `drawn`, one draw an item, each at its frame's entry in `table`
    /// and its row's in its batch's range from `rows_base`.
    ///
    /// # Panics
    ///
    /// If `kept` holds other than one item for each of `drawn`, was kept
    /// by another pipeline, or `table` is not the table this pipeline's
    /// kept draws bind (one table a pipeline).
    pub fn draw_kept(
        &mut self,
        target: &Target<'_>,
        kept: &KeptRange,
        (table, rows_base): (&mut TransformTable, u32),
        (entries, drawn): (&mut MeshEntries, &[&MeshEntry]),
    ) {
        self.binding.adopt(table, target.queue);
        self.draw_from(target, kept, (Some(table), rows_base), (entries, drawn));
    }

    /// Records the draws of `kept`'s items through the bind group over
    /// the slots and `table` (the identity table for `None`).
    fn draw_from(
        &mut self,
        target: &Target<'_>,
        kept: &KeptRange,
        (table, rows_base): (Option<&TransformTable>, u32),
        (entries, drawn): (&mut MeshEntries, &[&MeshEntry]),
    ) {
        assert_entries(self.kept.len(kept) / INSTANCE_BYTES, drawn.len());
        let Some(viewport) = self.binding.slot(target, rows_base, &self.work) else {
            return;
        };
        let run = Draws {
            pipeline: &self.pipeline,
            bind_group: self.binding.group(target, table, &self.work),
            viewport,
            items: self.kept.slice(target.encoder, kept),
            triangles: entries.slices(target, drawn),
        };
        run.record(target, &self.work, drawn);
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

    /// The bind groups kept: for the identity table and a painter's,
    /// each once drawn through.
    #[must_use]
    pub fn bind_groups_kept(&self) -> u64 {
        self.binding.kept()
    }

    /// The work its draws issued since the last take; the count restarts.
    pub fn take_work(&self) -> GpuWork {
        self.work.take()
    }
}

/// Refuses `count` kept items drawn with `entries` entries.
///
/// # Panics
///
/// When they differ: each item is drawn over its own entry.
fn assert_entries(count: u64, entries: usize) {
    assert_eq!(
        count, entries as u64,
        "meshes: {count} kept instances and {entries} entries; give each instance its entry"
    );
}

/// One run's draws: the pipeline, its bind group at `viewport`, the
/// run's items and each item's entry's triangles.
struct Draws<'a> {
    pipeline: &'a wgpu::RenderPipeline,
    bind_group: &'a wgpu::BindGroup,
    viewport: u32,
    items: wgpu::BufferSlice<'a>,
    triangles: Vec<wgpu::BufferSlice<'a>>,
}

impl Draws<'_> {
    /// Records one draw per item into `target`'s pass, the `i`th of
    /// item `i` over `drawn[i]`'s triangles, one instance each.
    fn record(self, target: &Target<'_>, work: &WorkCounter, drawn: &[&MeshEntry]) {
        target.draw_pass(|pass| {
            pass.set_pipeline(self.pipeline);
            pass.set_bind_group(0, self.bind_group, &[self.viewport]);
            for (at, (slice, entry)) in (0u64..).zip(self.triangles.into_iter().zip(drawn)) {
                pass.set_vertex_buffer(0, slice);
                pass.set_vertex_buffer(
                    1,
                    self.items
                        .slice(at * INSTANCE_BYTES..(at + 1) * INSTANCE_BYTES),
                );
                work.draw(pass, 0..3, 0..entry.triangles);
            }
        });
    }
}
