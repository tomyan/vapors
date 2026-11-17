//! What a pipeline that reads the transform table binds
//! (`docs/plans/flat-scene-f2.md`): the view uniform's slots (the
//! viewport and a batch's first row entry) and a table. Kept draws bind
//! one painter's table: a pipeline refuses a second, as it keeps one bind
//! group for it, keyed by the buffers' generations. Draws of the frame's
//! own items, which are on the page, bind the pipeline's identity table.
//! Shared by the instanced and mesh pipelines (`instanced.rs`,
//! `meshes.rs`), so the one-table rule has one answer.

use crate::instanced_bytes::{VIEW_SIZE, view_bytes};
use crate::kept_bind_group::KeptBindGroup;
use crate::uniform_slots::UniformSlots;
use crate::{Target, TransformTable, WorkCounter};

pub(crate) struct TableBinding {
    label: &'static str,
    layout: wgpu::BindGroupLayout,
    viewports: UniformSlots<VIEW_SIZE>,
    /// The identity alone, for draws of the frame's own items (entry 0,
    /// row 0).
    identity: TransformTable,
    own_group: KeptBindGroup,
    table_group: KeptBindGroup,
    /// The id of the table kept draws bind.
    table_id: Option<u64>,
}

impl TableBinding {
    /// The layout (`quads_setup.rs` `table_layout`), slots and identity
    /// table of the pipeline `label`.
    pub(crate) fn new(device: &wgpu::Device, label: &'static str) -> Self {
        Self {
            label,
            layout: crate::quads_setup::table_layout(device),
            viewports: UniformSlots::new(device, label),
            identity: TransformTable::new(device, label),
            own_group: KeptBindGroup::NONE,
            table_group: KeptBindGroup::NONE,
            table_id: None,
        }
    }

    /// The bind group layout, for the pipeline's own layout.
    pub(crate) fn layout(&self) -> &wgpu::BindGroupLayout {
        &self.layout
    }

    /// Takes `table` for a kept draw: written if it changed.
    ///
    /// # Panics
    ///
    /// When it is not the table this pipeline's kept draws bind.
    pub(crate) fn adopt(&mut self, table: &mut TransformTable, queue: &wgpu::Queue) {
        let id = *self.table_id.get_or_insert(table.id());
        assert_eq!(
            id,
            table.id(),
            "{}: a kept draw binds transform table {}, after table {id}: a pipeline draws from one \
             table, whose bind group it keeps (table_binding.rs); give each painter its own \
             pipelines; reproduce with `cargo xtask test`",
            self.label,
            table.id()
        );
        table.prepare(queue);
    }

    /// Writes the identity table on its first use, counted in `work`.
    pub(crate) fn prepare_identity(&mut self, queue: &wgpu::Queue, work: &WorkCounter) {
        self.identity.prepare_counted(queue, work);
    }

    /// The dynamic offset of the target's viewport and `rows_base` in the
    /// slots; `None` when refused (the refusal is on the encoder).
    pub(crate) fn slot(
        &mut self,
        target: &Target<'_>,
        rows_base: u32,
        work: &WorkCounter,
    ) -> Option<u32> {
        let value = view_bytes(target.viewport, rows_base);
        self.viewports
            .slot(target.encoder, target.queue, work, value)
    }

    /// The bind group over the slots and `table` (the identity table for
    /// `None`), made, counted in `work`, on first use and again when a
    /// buffer it names is replaced.
    pub(crate) fn group(
        &mut self,
        target: &Target<'_>,
        table: Option<&TransformTable>,
        work: &WorkCounter,
    ) -> &wgpu::BindGroup {
        let (kept, table) = match table {
            Some(table) => (&mut self.table_group, table),
            None => (&mut self.own_group, &self.identity),
        };
        let generation = (self.viewports.generation(), table.generation());
        let (label, layout, viewports) = (self.label, &self.layout, &self.viewports);
        kept.of(generation, || {
            made_group(target.device, (label, layout, work), (viewports, table))
        })
    }

    /// The bind groups kept: one for the identity table and one for a
    /// painter's, each once drawn through.
    pub(crate) fn kept(&self) -> u64 {
        self.own_group.kept() + self.table_group.kept()
    }
}

fn made_group(
    device: &wgpu::Device,
    (label, layout, work): (&str, &wgpu::BindGroupLayout, &WorkCounter),
    (viewports, table): (&UniformSlots<VIEW_SIZE>, &TransformTable),
) -> wgpu::BindGroup {
    work.create_bind_group(
        device,
        &wgpu::BindGroupDescriptor {
            label: Some(label),
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
