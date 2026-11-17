//! What a pipeline that reads the transform table binds
//! (`docs/plans/flat-scene-f2.md`): the view uniform's slots (the
//! viewport and a batch's first row entry) and a table. Kept draws bind
//! one painter's table: a pipeline refuses a second, as it keeps one bind
//! group for it, keyed by the buffers' generations. Draws of the frame's
//! own items, which are on the page, bind the pipeline's identity table.
//! Shared by the instanced and mesh pipelines (`instanced.rs`,
//! `meshes.rs`), so the one-table rule has one answer. The instanced
//! pipelines bind the clip table too (flat-scene F3.1, `clip.wgsl`),
//! held to one table the same way; their own draws bind a clip table of
//! no clips.

use crate::instanced_bytes::{VIEW_SIZE, view_bytes};
use crate::kept_bind_group::KeptBindGroup;
use crate::uniform_slots::UniformSlots;
use crate::{ClipTable, Target, TransformTable, WorkCounter};

/// A kept draw's tables: the transform table, and the clip table for a
/// pipeline that binds one.
pub(crate) type Tables<'a> = (&'a TransformTable, Option<&'a ClipTable>);

pub(crate) struct TableBinding {
    label: &'static str,
    layout: wgpu::BindGroupLayout,
    viewports: UniformSlots<VIEW_SIZE>,
    /// The identity alone, for draws of the frame's own items (entry 0,
    /// row 0).
    identity: TransformTable,
    /// The clip table of no clips the frame's own items bind, for a
    /// pipeline that binds clips.
    no_clips: Option<ClipTable>,
    own_group: KeptBindGroup<[u64; 3]>,
    table_group: KeptBindGroup<[u64; 3]>,
    /// The ids of the transform and clip tables kept draws bind.
    table_id: Option<u64>,
    clips_id: Option<u64>,
}

impl TableBinding {
    /// The layout (`quads_setup.rs` `table_layout`, or `tables_layout`
    /// with the clip table when `clips`), slots and identity table of the
    /// pipeline `label`.
    pub(crate) fn new(device: &wgpu::Device, label: &'static str, clips: bool) -> Self {
        let layout = if clips {
            crate::quads_setup::tables_layout(device)
        } else {
            crate::quads_setup::table_layout(device)
        };
        Self {
            label,
            layout,
            viewports: UniformSlots::new(device, label),
            identity: TransformTable::new(device, label),
            no_clips: clips.then(|| ClipTable::new(device, label)),
            own_group: KeptBindGroup::NONE,
            table_group: KeptBindGroup::NONE,
            table_id: None,
            clips_id: None,
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
    pub(crate) fn adopt(
        &mut self,
        (table, clips): (&mut TransformTable, Option<&mut ClipTable>),
        queue: &wgpu::Queue,
    ) {
        if let Some(clips) = clips {
            self.adopt_clips(clips, queue);
        }
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

    /// Takes `clips` for a kept draw: written if it changed.
    ///
    /// # Panics
    ///
    /// When it is not the clip table this pipeline's kept draws bind.
    fn adopt_clips(&mut self, clips: &mut ClipTable, queue: &wgpu::Queue) {
        let id = *self.clips_id.get_or_insert(clips.id());
        assert_eq!(
            id,
            clips.id(),
            "{}: a kept draw binds clip table {}, after table {id}: a pipeline draws from one \
             clip table, as from one transform table (table_binding.rs); give each painter its \
             own pipelines; reproduce with `cargo xtask test`",
            self.label,
            clips.id()
        );
        clips.prepare(queue);
    }

    /// Writes the identity table, and the clip table of no clips, on
    /// their first use, counted in `work`.
    pub(crate) fn prepare_identity(&mut self, queue: &wgpu::Queue, work: &WorkCounter) {
        self.identity.prepare_counted(queue, work);
        if let Some(no_clips) = &mut self.no_clips {
            no_clips.prepare_counted(queue, work);
        }
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

    /// The bind group over the slots and `tables` (the identity table and
    /// the clip table of no clips for `None`), made, counted in `work`, on
    /// first use and again when a buffer it names is replaced.
    pub(crate) fn group(
        &mut self,
        target: &Target<'_>,
        tables: Option<Tables<'_>>,
        work: &WorkCounter,
    ) -> &wgpu::BindGroup {
        let (kept, (table, clips)) = match tables {
            Some(tables) => (&mut self.table_group, tables),
            None => (
                &mut self.own_group,
                (&self.identity, self.no_clips.as_ref()),
            ),
        };
        let clips_generation = clips.map_or(0, ClipTable::generation);
        let generation = [
            self.viewports.generation(),
            table.generation(),
            clips_generation,
        ];
        let (label, layout, viewports) = (self.label, &self.layout, &self.viewports);
        kept.of(generation, || {
            made_group(
                target.device,
                (label, layout, work),
                (viewports, table, clips),
            )
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
    (viewports, table, clips): (
        &UniformSlots<VIEW_SIZE>,
        &TransformTable,
        Option<&ClipTable>,
    ),
) -> wgpu::BindGroup {
    let entry = |binding, resource| wgpu::BindGroupEntry { binding, resource };
    let mut entries = vec![entry(0, viewports.binding()), entry(1, table.binding())];
    if let Some(clips) = clips {
        entries.push(entry(2, clips.binding()));
    }
    work.create_bind_group(
        device,
        &wgpu::BindGroupDescriptor {
            label: Some(label),
            layout,
            entries: &entries,
        },
    )
}
