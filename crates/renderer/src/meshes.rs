//! Drawing a run of mesh items (docs/plans/one-scene.md 1.2): each the
//! triangles of an entry in [`MeshEntries`] under its own transform and
//! colour, one draw per item, in order, one instance per triangle. The
//! vertex shader moves each outline corner half a device pixel out
//! across both lines of its wedge, so the ramp straddles the outline;
//! the fragment shader takes coverage from the triangle's corners'
//! wedges in device pixels. Instance bytes are kept between frames as a
//! quad run's are; the entries' triangles are the store's.

use crate::instanced::float_bytes;
use crate::instanced_bytes::view_bytes;
use crate::kept_bind_group::KeptBindGroup;
use crate::mesh_entries::{MeshEntries, MeshEntry};
use crate::uniform_slots::UniformSlots;
use crate::{GpuWork, KeptRange, MeshShape, Retained, Target, TransformTable, WorkCounter};

/// Bytes per item: [`MeshShape::instance`]'s floats, each draw's slice
/// of the kept buffer, which `meshes_setup.rs` reads at stride 0.
pub(crate) const INSTANCE_BYTES: u64 = MeshPipeline::INSTANCE_BYTES as u64;

/// A pipeline that draws mesh items. Create once per target format.
pub struct MeshPipeline {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    viewports: UniformSlots,
    /// Over the slots and the pipeline's own identity table, for the
    /// draws of this frame's items, which are on the page.
    own_group: KeptBindGroup,
    /// Over the slots and a painter's transform table.
    table_group: KeptBindGroup,
    identity: TransformTable,
    /// The id of the table kept draws bind: one table a pipeline.
    table_id: Option<u64>,
    kept: Retained,
    work: WorkCounter,
}

impl MeshPipeline {
    /// Bytes per item in the kept buffer: [`MeshShape::instance`]'s
    /// floats, then its row's index in its batch's range and its frame's
    /// entry in the transform table, padded to 16 bytes as kept bytes are
    /// (`retained.rs`).
    pub const INSTANCE_BYTES: usize = crate::mesh_shape::INSTANCE_FLOATS * 4 + 16;

    /// Build the pipeline for targets of `format`.
    #[must_use]
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let layout = crate::quads_setup::table_layout(device);
        Self {
            pipeline: crate::meshes_setup::render_pipeline(device, &layout, format),
            layout,
            viewports: UniformSlots::new(device, "meshes"),
            own_group: KeptBindGroup::NONE,
            table_group: KeptBindGroup::NONE,
            identity: TransformTable::new(device, "mesh identity"),
            table_id: None,
            kept: Retained::new(device, "meshes", wgpu::BufferUsages::VERTEX),
            work: WorkCounter::default(),
        }
    }

    /// The item bytes of `shapes`, as a draw keeps them: each on the
    /// page.
    #[must_use]
    pub fn bytes(shapes: &[MeshShape]) -> Vec<u8> {
        Self::bytes_in(shapes, 0, &[])
    }

    /// The item bytes of `shapes`, each in frame `frame` (its entry in
    /// the transform table), item `i` in row `rows[i]` of its batch's
    /// range, or every item in row 0 for an empty `rows`.
    ///
    /// # Panics
    ///
    /// As [`QuadPipeline::bytes_in`](crate::QuadPipeline::bytes_in).
    #[must_use]
    pub fn bytes_in(shapes: &[MeshShape], frame: u32, rows: &[u32]) -> Vec<u8> {
        crate::instanced_bytes::assert_rows("meshes", shapes.len(), rows);
        let mut bytes = Vec::with_capacity(shapes.len() * Self::INSTANCE_BYTES);
        for (at, shape) in shapes.iter().enumerate() {
            bytes.extend(float_bytes(shape.instance()));
            bytes.extend(rows.get(at).copied().unwrap_or(0).to_le_bytes());
            bytes.extend(frame.to_le_bytes());
            bytes.extend([0; 8]);
        }
        bytes
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
        self.identity.prepare_counted(target.queue, &self.work);
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
        let id = *self.table_id.get_or_insert(table.id());
        assert_eq!(
            id,
            table.id(),
            "meshes: a kept draw binds transform table {}, after table {id}: a pipeline draws \
             from one table (meshes.rs); give each painter its own pipelines; reproduce with \
             `cargo xtask test`",
            table.id()
        );
        table.prepare(target.queue);
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
        let count = self.kept.len(kept) / INSTANCE_BYTES;
        assert_eq!(
            count,
            drawn.len() as u64,
            "meshes: {count} kept instances and {} entries; give each instance its entry",
            drawn.len()
        );
        let value = view_bytes(target.viewport, rows_base);
        let Some(viewport) = self
            .viewports
            .slot(target.encoder, target.queue, &self.work, value)
        else {
            return;
        };
        let (kept_group, table) = match table {
            Some(table) => (&mut self.table_group, table),
            None => (&mut self.own_group, &self.identity),
        };
        let (layout, viewports, work) = (&self.layout, &self.viewports, &self.work);
        let generation = (viewports.generation(), table.generation());
        let bind_group = kept_group.of(generation, || {
            made_bind_group(target, layout, work, (viewports, table))
        });
        let run = Draws {
            pipeline: &self.pipeline,
            bind_group,
            viewport,
            items: self.kept.slice(target.encoder, kept),
            triangles: entries.slices(target, drawn),
        };
        run.record(target, work, drawn);
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
        self.own_group.kept() + self.table_group.kept()
    }

    /// The work its draws issued since the last take; the count restarts.
    pub fn take_work(&self) -> GpuWork {
        self.work.take()
    }
}

fn made_bind_group(
    target: &Target<'_>,
    layout: &wgpu::BindGroupLayout,
    work: &WorkCounter,
    (viewports, table): (&UniformSlots, &TransformTable),
) -> wgpu::BindGroup {
    work.create_bind_group(
        target.device,
        &wgpu::BindGroupDescriptor {
            label: Some("meshes"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: viewports.binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: table.binding(),
                },
            ],
        },
    )
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

#[cfg(test)]
#[path = "meshes_bytes_tests.rs"]
mod bytes_tests;
