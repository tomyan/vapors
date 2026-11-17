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
use crate::quads_setup::render_pipeline;
use crate::table_binding::TableBinding;
use crate::{KeptRange, Retained, Target, TransformTable, WorkCounter};

const VERTICES_PER_QUAD: u32 = 6;

pub(crate) struct Instanced {
    label: &'static str,
    pipeline: wgpu::RenderPipeline,
    binding: TableBinding,
    instances: FrameArena,
    kept: Retained,
    stride: u64,
    pub(crate) work: WorkCounter,
}

/// A kept draw's transform table and its batch's first row entry.
pub(crate) type Table<'a> = (&'a mut TransformTable, u32);

impl Instanced {
    pub(crate) fn new(
        device: &wgpu::Device,
        pass: crate::PassFormat,
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
        let binding = TableBinding::new(device, label);
        let stride = instance_bytes(vec4s) as u64;
        let attributes = attributes(vec4s);
        let instance = wgpu::VertexBufferLayout {
            array_stride: stride,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &attributes,
        };
        Self {
            label,
            pipeline: render_pipeline(device, binding.layout(), &shader, pass, instance),
            binding,
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
        let Some(viewport) = self.binding.slot(target, 0, &self.work) else {
            return;
        };
        self.binding.prepare_identity(queue, &self.work);
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
        (table, rows_base): Table<'_>,
    ) {
        let count = self.count(self.kept.len(kept));
        let Some(viewport) = self.binding.slot(target, rows_base, &self.work) else {
            return;
        };
        self.binding.adopt(table, target.queue);
        self.record(
            target,
            (count, Instances::Kept(kept)),
            (Some(table), viewport),
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

    /// The bind groups kept: for the identity table and a painter's.
    pub(crate) fn bind_groups_kept(&self) -> u64 {
        self.binding.kept()
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

    /// Records the draw of `count` instances from `from`, through the
    /// bind group over the slots and `table` (the pipeline's identity
    /// table for `None`), the viewport slot at `viewport`. Each bind group
    /// is made on first use and again when a buffer it names is replaced.
    fn record(
        &mut self,
        target: &Target<'_>,
        (count, from): (u32, Instances<'_>),
        (table, viewport): (Option<&TransformTable>, u32),
    ) {
        let _encoding = tracing::trace_span!("draw step", phase = "encode").entered();
        let instances = match from {
            Instances::Placed(placed) => placed.slice(),
            Instances::Kept(kept) => self.kept.slice(target.encoder, kept),
        };
        let group = self.binding.group(target, table, &self.work);
        target.draw_pass(|pass| {
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, group, &[viewport]);
            pass.set_vertex_buffer(0, instances);
            self.work.draw(pass, 0..VERTICES_PER_QUAD, 0..count);
        });
    }
}

/// The instance attributes: `vec4s` four-float ones, then the row's index
/// and the frame's entry, two `u32`s.
fn attributes(vec4s: u32) -> Vec<wgpu::VertexAttribute> {
    let floats = u64::from(vec4s) * 16;
    let index = |at: u32| wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Uint32,
        offset: floats + u64::from(at) * 4,
        shader_location: vec4s + at,
    };
    (0..vec4s)
        .map(|i| wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x4,
            offset: u64::from(i) * 16,
            shader_location: i,
        })
        .chain([index(0), index(1)])
        .collect()
}

/// Where a draw's instances are: this frame's arena, or kept.
#[derive(Clone, Copy)]
enum Instances<'a> {
    Placed(&'a Placed),
    Kept(&'a KeptRange),
}

pub(crate) use crate::instanced_bytes::{float_bytes, instance_bytes, rowed_instances};

#[cfg(test)]
#[path = "instanced_layout_tests.rs"]
mod layout_tests;

#[cfg(test)]
#[path = "instanced_tests.rs"]
mod tests;
