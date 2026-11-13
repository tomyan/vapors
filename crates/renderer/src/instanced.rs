//! A pipeline that draws one quad per instance: the shared machinery of
//! the box and shadow pipelines. Each instance is `vec4s` four-float
//! attributes, in device pixels where they are positions; the viewport
//! size and a scroll frame's shift (device pixels, added to every
//! position) are a uniform, kept per value seen and bound by its offset. Alpha
//! blends over what the target holds, in instance order, in one draw
//! recorded into the target's encoder.

use vapors_scene::Size;

use crate::frame_arena::{FrameArena, Placed};
use crate::kept_bind_group::KeptBindGroup;
use crate::quads_setup::{bind_group_layout, render_pipeline};
use crate::uniform_slots::UniformSlots;
use crate::{KeptRange, Retained, Target, WorkCounter};

const VERTICES_PER_QUAD: u32 = 6;

pub(crate) struct Instanced {
    label: &'static str,
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    viewports: UniformSlots,
    bind_group: KeptBindGroup,
    instances: FrameArena,
    kept: Retained,
    stride: u64,
    pub(crate) work: WorkCounter,
}

impl Instanced {
    pub(crate) fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        label: &'static str,
        source: &str,
        vec4s: u32,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(label),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let layout = bind_group_layout(device);
        let stride = u64::from(vec4s) * 16;
        let attributes: Vec<wgpu::VertexAttribute> = (0..vec4s)
            .map(|i| wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x4,
                offset: u64::from(i) * 16,
                shader_location: i,
            })
            .collect();
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
            bind_group: KeptBindGroup::NONE,
            instances: FrameArena::new(device, label, wgpu::BufferUsages::VERTEX, stride),
            kept: Retained::new(device, label, wgpu::BufferUsages::VERTEX),
            stride,
            work: WorkCounter::default(),
        }
    }

    /// Draws the instances whose attributes are `bytes`; a draw its
    /// arenas refuse is not recorded (the refusal is on the encoder).
    pub(crate) fn draw(&mut self, target: &Target<'_>, bytes: &[u8]) {
        let count = self.count(bytes.len() as u64);
        if count == 0 {
            return;
        }
        let (frame, queue) = (target.encoder, target.queue);
        let Some(instances) = self.instances.push(frame, queue, &self.work, bytes) else {
            return;
        };
        let Some(viewport) = self.viewport_slot(target, [0.0; 2]) else {
            return;
        };
        self.record(target, count, Instances::Placed(&instances), viewport);
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

    /// Draws the instances `kept` holds, moved by `shift` (device
    /// pixels); not when the viewport slot is refused (the refusal is on
    /// the encoder).
    pub(crate) fn draw_kept(&mut self, target: &Target<'_>, kept: &KeptRange, shift: [f32; 2]) {
        let count = self.count(self.kept.len(kept));
        let Some(viewport) = self.viewport_slot(target, shift) else {
            return;
        };
        self.record(target, count, Instances::Kept(kept), viewport);
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
        self.bind_group.kept()
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

    /// The dynamic offset of the target's viewport and `shift` in the
    /// slots.
    fn viewport_slot(&mut self, target: &Target<'_>, shift: [f32; 2]) -> Option<u32> {
        let viewport = viewport_bytes(target.viewport, shift);
        let (frame, queue) = (target.encoder, target.queue);
        self.viewports.slot(frame, queue, &self.work, viewport)
    }

    /// Records the draw with the viewport slot at `viewport`, through
    /// the kept bind group, made for the slots' buffer on first use and
    /// again when the buffer is replaced.
    fn record(&mut self, target: &Target<'_>, count: u32, from: Instances<'_>, viewport: u32) {
        let _encoding = tracing::trace_span!("draw step", phase = "encode").entered();
        let (label, layout, work, viewports) =
            (self.label, &self.layout, &self.work, &self.viewports);
        let bind_group = self.bind_group.of(viewports.generation(), || {
            work.create_bind_group(
                target.device,
                &wgpu::BindGroupDescriptor {
                    label: Some(label),
                    layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: viewports.binding(),
                    }],
                },
            )
        });
        let instances = match from {
            Instances::Placed(placed) => placed.slice(),
            Instances::Kept(kept) => self.kept.slice(target.encoder, kept),
        };
        target.draw_pass(|pass| {
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, bind_group, &[viewport]);
            pass.set_vertex_buffer(0, instances);
            self.work.draw(pass, 0..VERTICES_PER_QUAD, 0..count);
        });
    }
}

/// Where a draw's instances are: this frame's arena, or kept.
#[derive(Clone, Copy)]
enum Instances<'a> {
    Placed(&'a Placed),
    Kept(&'a KeptRange),
}

/// The viewport uniform's bytes: its size and `shift`, two floats each.
#[allow(clippy::cast_precision_loss)] // viewports are far below 2^24 pixels
pub(crate) fn viewport_bytes(viewport: Size, [dx, dy]: [f32; 2]) -> [u8; 16] {
    let floats = [viewport.width as f32, viewport.height as f32, dx, dy];
    let mut bytes = [0; 16];
    for (at, float) in floats.into_iter().enumerate() {
        bytes[at * 4..at * 4 + 4].copy_from_slice(&float.to_le_bytes());
    }
    bytes
}

/// Little-endian bytes of `floats`, the instance buffer's layout.
pub(crate) fn float_bytes(floats: impl IntoIterator<Item = f32>) -> Vec<u8> {
    floats.into_iter().flat_map(f32::to_le_bytes).collect()
}
