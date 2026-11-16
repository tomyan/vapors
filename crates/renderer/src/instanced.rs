//! A pipeline that draws one quad per instance: the shared machinery of
//! the box and shadow pipelines. Each instance is `vec4s` four-float
//! attributes, in device pixels where they are positions, then its row's
//! index (`row_offsets.rs`); the viewport size, a frame's shift and the
//! rows' offsets (device pixels, added to every position) are a
//! uniform, kept per value seen and bound by its offset. Alpha
//! blends over what the target holds, in instance order, in one draw
//! recorded into the target's encoder.

use vapors_scene::Size;

use crate::frame_arena::{FrameArena, Placed};
use crate::kept_bind_group::KeptBindGroup;
use crate::quads_setup::{bind_group_layout, render_pipeline};
use crate::uniform_slots::UniformSlots;
use crate::{KeptRange, Retained, RowOffsets, Shift, Target, WorkCounter};

const VERTICES_PER_QUAD: u32 = 6;

pub(crate) struct Instanced {
    label: &'static str,
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    viewports: UniformSlots<VIEW_SIZE>,
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
        let shader = crate::pipelines::shader_module(
            device,
            wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            },
        );
        let layout = bind_group_layout(device);
        let floats = u64::from(vec4s) * 16;
        let stride = instance_bytes(vec4s) as u64;
        let row = wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Uint32,
            offset: floats,
            shader_location: vec4s,
        };
        let attributes: Vec<wgpu::VertexAttribute> = (0..vec4s)
            .map(|i| wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x4,
                offset: u64::from(i) * 16,
                shader_location: i,
            })
            .chain(std::iter::once(row))
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
        let Some(viewport) = self.viewport_slot(target, (Shift::NONE, &RowOffsets::NONE)) else {
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

    /// Draws the instances `kept` holds, moved by `shift` and each by its
    /// row's offset in `rows` (device pixels); not when the viewport slot
    /// is refused (the refusal is on the encoder).
    pub(crate) fn draw_kept(
        &mut self,
        target: &Target<'_>,
        kept: &KeptRange,
        moved: (Shift, &RowOffsets),
    ) {
        let count = self.count(self.kept.len(kept));
        let Some(viewport) = self.viewport_slot(target, moved) else {
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

    /// The dynamic offset of the target's viewport, `shift` and `rows`
    /// in the slots.
    fn viewport_slot(
        &mut self,
        target: &Target<'_>,
        (shift, rows): (Shift, &RowOffsets),
    ) -> Option<u32> {
        let head = viewport_bytes(target.viewport, shift);
        let (frame, queue) = (target.encoder, target.queue);
        self.viewports
            .slot_with_rows(frame, (queue, &self.work), (head, rows))
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

/// Bytes per instance of `vec4s` four-float attributes: the attributes and
/// the row index after them, padded to 16 bytes, as kept instances are
/// whole 16-byte rows (`retained.rs`).
pub(crate) const fn instance_bytes(vec4s: u32) -> usize {
    (vec4s as usize + 1) * 16
}

/// Little-endian bytes of `floats`: a pipeline's instances with no row
/// index (the mesh pipeline's and the scene's closed forms' own layouts,
/// `meshes.rs`, `closed_forms_3d.rs`).
pub(crate) fn float_bytes(floats: impl IntoIterator<Item = f32>) -> Vec<u8> {
    floats.into_iter().flat_map(f32::to_le_bytes).collect()
}

/// The viewport uniform's bytes for a pipeline that draws one item a
/// draw (images): its size and `shift`, two floats each; an item's row
/// offset is in its shift.
#[allow(clippy::cast_precision_loss)] // viewports are far below 2^24 pixels
pub(crate) fn viewport_bytes(viewport: Size, shift: Shift) -> [u8; 16] {
    let [dx, dy] = shift.pixels();
    let floats = [viewport.width as f32, viewport.height as f32, dx, dy];
    let mut bytes = [0; 16];
    for (at, float) in floats.into_iter().enumerate() {
        bytes[at * 4..at * 4 + 4].copy_from_slice(&float.to_le_bytes());
    }
    bytes
}

/// The view uniform's bytes: the viewport size and `shift`, two floats
/// each, then the rows' offsets (`RowOffsets::bytes`).
pub(crate) const VIEW_SIZE: usize = 16 + RowOffsets::CAPACITY * 8;

/// The instance bytes of `items` for the pipeline `label`: each item's
/// `vec4s` four-float attributes (`floats`), then its row's index in
/// `rows`, padded to 16 bytes. An empty `rows` puts every item in row 0
/// (a batch in one frame).
///
/// # Panics
///
/// When `rows` is neither empty nor one index an item, when an index is
/// past the [`RowOffsets`] table (the shader would read it clamped,
/// silently), when an item's floats are not `vec4s` four-float
/// attributes, or when the bytes built are not [`instance_bytes`] an
/// item, the stride the pipeline reads them at.
pub(crate) fn rowed_instances<T>(
    (label, vec4s): (&str, usize),
    (items, rows): (&[T], &[u32]),
    floats: impl Fn(&T) -> Vec<f32>,
) -> Vec<u8> {
    assert_rows(label, items.len(), rows);
    let mut bytes = Vec::new();
    for (at, item) in items.iter().enumerate() {
        let floats = floats(item);
        assert_eq!(
            floats.len(),
            vec4s * 4,
            "{label}: an instance is {vec4s} vec4s"
        );
        bytes.extend(floats.into_iter().flat_map(f32::to_le_bytes));
        bytes.extend(rows.get(at).copied().unwrap_or(0).to_le_bytes());
        bytes.extend([0; 12]);
    }
    #[allow(clippy::cast_possible_truncation)] // a handful of vec4s
    let instance = instance_bytes(vec4s as u32);
    assert_eq!(
        bytes.len(),
        items.len() * instance,
        "{label}: instances built at other than the {instance} bytes the pipeline reads \
         (instanced.rs `instance_bytes`)"
    );
    bytes
}

/// Refuses, naming `label`, rows that are not empty or one index for
/// each of `items` items, every one inside the [`RowOffsets`] table.
///
/// # Panics
///
/// As said: an index past the table would be read clamped, silently, and
/// a short `rows` would put the rest in row 0.
pub fn assert_rows(label: &str, items: usize, rows: &[u32]) {
    assert!(
        rows.is_empty() || rows.len() == items,
        "{label}: {} row indexes for {items} items; give one an item, or none for a batch in one \
         frame (instanced.rs); reproduce with `cargo xtask test`",
        rows.len()
    );
    let past = rows
        .iter()
        .position(|&row| row as usize >= RowOffsets::CAPACITY);
    if let Some(at) = past {
        panic!(
            "{label}: item {at} names row {}, past the {} rows a batch's table holds; split the \
             run into batches of at most that many rows (row_offsets.rs); reproduce with \
             `cargo xtask test`",
            rows[at],
            RowOffsets::CAPACITY
        );
    }
}

#[cfg(test)]
#[path = "instanced_layout_tests.rs"]
mod layout_tests;

#[cfg(test)]
#[path = "instanced_tests.rs"]
mod tests;
