//! A pipeline that draws one quad per instance: the shared machinery of
//! the box, shadow and closed-form pipelines. Each instance is `vec4s`
//! four-float attributes, in device pixels where they are positions,
//! then its row's index in its batch's range and its frame's entry in
//! the transform table (`transform_table.rs`,
//! `docs/plans/flat-scene-f2.md`), which the vertex stage reads. The
//! viewport size and the range's first entry are a uniform, kept per
//! value seen and bound by its offset. Alpha blends over what the target
//! holds, in instance order, in one draw recorded into the target's
//! encoder.

use crate::frame_arena::{FrameArena, Placed};
use crate::instanced_bytes::view_bytes;
use crate::kept_bind_group::KeptBindGroup;
use crate::quads_setup::{render_pipeline, tables_layout};
use crate::uniform_slots::UniformSlots;
use crate::{ClipTable, KeptRange, Retained, Target, TransformTable, WorkCounter};

const VERTICES_PER_QUAD: u32 = 6;

pub(crate) struct Instanced {
    label: &'static str,
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    viewports: UniformSlots<VIEW_SIZE>,
    /// The identity alone, for draws of this frame's instances, which are
    /// on the page (entry 0, row 0).
    identity: TransformTable,
    /// The clip table of no clips its own draws bind.
    no_clips: ClipTable,
    own_group: KeptBindGroup<[u64; 3]>,
    table_group: KeptBindGroup<[u64; 3]>,
    /// The id of the table kept draws bind: one table a pipeline, as one
    /// bind group is kept for it, keyed by its generation alone.
    table_id: Option<u64>,
    /// The id of the clip table kept draws bind, held to one as the
    /// transform table's is.
    clips_id: Option<u64>,
    instances: FrameArena,
    kept: Retained,
    stride: u64,
    pub(crate) work: WorkCounter,
}

/// A kept draw's transform table, clip table, and its batch's first row
/// entry.
pub(crate) type Table<'a> = (&'a mut TransformTable, &'a mut ClipTable, u32);

impl Instanced {
    pub(crate) fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        label: &'static str,
        source: &str,
        vec4s: u32,
    ) -> Self {
        let shader = crate::pipelines::shader_module(
            device,
            wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            },
        );
        let layout = tables_layout(device);
        let stride = instance_bytes(vec4s) as u64;
        let attributes = attributes(vec4s);
        let instance = wgpu::VertexBufferLayout {
            array_stride: stride,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &attributes,
        };
        Self {
            label,
            pipeline: render_pipeline(device, &layout, &shader, format, instance),
            layout,
            viewports: UniformSlots::new(device, label),
            identity: TransformTable::new(device, label),
            no_clips: ClipTable::new(device, label),
            own_group: KeptBindGroup::NONE,
            table_group: KeptBindGroup::NONE,
            table_id: None,
            clips_id: None,
            instances: FrameArena::new(device, label, wgpu::BufferUsages::VERTEX, stride),
            kept: Retained::new(device, label, wgpu::BufferUsages::VERTEX),
            stride,
            work: WorkCounter::default(),
        }
    }

    /// Draws the instances whose attributes are `bytes`, each on the page;
    /// a draw its arenas refuse is not recorded (the refusal is on the
    /// encoder).
    pub(crate) fn draw(&mut self, target: &Target<'_>, bytes: &[u8]) {
        let count = self.count(bytes.len() as u64);
        if count == 0 {
            return;
        }
        let (frame, queue) = (target.encoder, target.queue);
        let Some(instances) = self.instances.push(frame, queue, &self.work, bytes) else {
            return;
        };
        let Some(viewport) = self.viewport_slot(target, 0) else {
            return;
        };
        self.identity.prepare_counted(queue, &self.work);
        self.no_clips.prepare_counted(queue, &self.work);
        self.record(
            target,
            (count, Instances::Placed(&instances)),
            (None, viewport),
        );
    }

    /// Writes the instances' `bytes` into the kept buffer, for
    /// [`Instanced::draw_kept`] in this and later frames; `None` when
    /// it refuses them (the refusal is on the encoder).
    ///
    /// # Panics
    ///
    /// If `bytes` are not a whole number of instances.
    pub(crate) fn keep(&mut self, target: &Target<'_>, bytes: &[u8]) -> Option<KeptRange> {
        let len = bytes.len() as u64;
        assert!(
            len.is_multiple_of(self.stride),
            "{}: {len} kept bytes, not a whole number of {}-byte instances; keep the \
             pipeline's bytes() of a run (instanced.rs)",
            self.label,
            self.stride
        );
        let (frame, queue) = (target.encoder, target.queue);
        self.kept.keep(frame, queue, &self.work, bytes)
    }

    /// Draws the instances `kept` holds, each at its frame's entry in
    /// `table` and its row's in the range from `rows_base`; not when the
    /// viewport slot is refused (the refusal is on the encoder).
    pub(crate) fn draw_kept(
        &mut self,
        target: &Target<'_>,
        kept: &KeptRange,
        (table, clips, rows_base): Table<'_>,
    ) {
        let count = self.count(self.kept.len(kept));
        let Some(viewport) = self.viewport_slot(target, rows_base) else {
            return;
        };
        let id = *self.table_id.get_or_insert(table.id());
        assert_eq!(
            id,
            table.id(),
            "{}: a kept draw binds transform table {}, after table {id}: a pipeline draws from one \
             table, whose bind group it keeps (instanced.rs); give each painter its own pipelines; \
             reproduce with `cargo xtask test`",
            self.label,
            table.id()
        );
        let clips_id = *self.clips_id.get_or_insert(clips.id());
        assert_eq!(
            clips_id,
            clips.id(),
            "{}: a kept draw binds clip table {}, after table {clips_id}: a pipeline draws from one \
             clip table, whose bind group it keeps (instanced.rs); give each painter its own \
             pipelines; reproduce with `cargo xtask test`",
            self.label,
            clips.id()
        );
        table.prepare(target.queue);
        clips.prepare(target.queue);
        self.record(
            target,
            (count, Instances::Kept(kept)),
            (Some((table, clips)), viewport),
        );
    }

    /// Gives `kept`'s bytes back to the kept buffer.
    pub(crate) fn release(&mut self, kept: KeptRange) {
        self.kept.release(kept);
    }

    /// Bytes held in the kept buffer.
    pub(crate) fn retained_bytes(&self) -> u64 {
        self.kept.live_bytes()
    }

    /// The bind groups kept: one once this pipeline has drawn.
    pub(crate) fn bind_groups_kept(&self) -> u64 {
        self.own_group.kept() + self.table_group.kept()
    }

    /// How many instances `bytes` bytes hold.
    fn count(&self, bytes: u64) -> u32 {
        u32::try_from(bytes / self.stride).unwrap_or_else(|_| {
            panic!(
                "{}: more instances in one batch than fit one draw",
                self.label
            )
        })
    }

    /// The dynamic offset of the target's viewport and `rows_base` in the
    /// slots.
    fn viewport_slot(&mut self, target: &Target<'_>, rows_base: u32) -> Option<u32> {
        let value = view_bytes(target.viewport, rows_base);
        let (frame, queue) = (target.encoder, target.queue);
        self.viewports.slot(frame, queue, &self.work, value)
    }

    /// Records the draw of `count` instances from `from`, through the
    /// bind group over the slots and `tables` (the pipeline's identity
    /// table and empty clip table for `None`), the viewport slot at
    /// `viewport`. Each bind group
    /// is made on first use and again when a buffer it names is replaced.
    fn record(
        &mut self,
        target: &Target<'_>,
        (count, from): (u32, Instances<'_>),
        (tables, viewport): (Option<(&TransformTable, &ClipTable)>, u32),
    ) {
        let _encoding = tracing::trace_span!("draw step", phase = "encode").entered();
        let group = self.bind_group(target.device, tables);
        let instances = match from {
            Instances::Placed(placed) => placed.slice(),
            Instances::Kept(kept) => self.kept.slice(target.encoder, kept),
        };
        target.draw_pass(|pass| {
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &group, &[viewport]);
            pass.set_vertex_buffer(0, instances);
            self.work.draw(pass, 0..VERTICES_PER_QUAD, 0..count);
        });
    }

    /// The bind group of the viewports and `tables` (this pipeline's own
    /// identity and empty clip table for none), made when one of their
    /// buffers is new; a handle to the kept one.
    fn bind_group(
        &mut self,
        device: &wgpu::Device,
        tables: Option<(&TransformTable, &ClipTable)>,
    ) -> wgpu::BindGroup {
        let (kept_group, (table, clips)) = match tables {
            Some(tables) => (&mut self.table_group, tables),
            None => (&mut self.own_group, (&self.identity, &self.no_clips)),
        };
        let generations = [
            self.viewports.generation(),
            table.generation(),
            clips.generation(),
        ];
        let (label, layout, work) = (self.label, &self.layout, &self.work);
        let viewports = &self.viewports;
        kept_group
            .of(generations, || {
                group(device, (label, layout, work), (viewports, table, clips))
            })
            .clone()
    }
}

/// Where a draw's instances are: this frame's arena, or kept.
#[derive(Clone, Copy)]
enum Instances<'a> {
    Placed(&'a Placed),
    Kept(&'a KeptRange),
}

pub(crate) use crate::instanced_binding::instanced_source;
use crate::instanced_binding::{attributes, group};
pub(crate) use crate::instanced_bytes::{
    VIEW_SIZE, float_bytes, instance_bytes, rowed_instances, viewport_bytes,
};

#[cfg(test)]
#[path = "instanced_layout_tests.rs"]
mod layout_tests;

#[cfg(test)]
#[path = "instanced_tests.rs"]
mod tests;
