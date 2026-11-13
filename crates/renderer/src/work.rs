//! GPU work counted on the CPU where it is issued: bytes handed to the
//! queue's writes and draw calls recorded, with no readback and no wait.

use std::ops::{Add, AddAssign};
use std::sync::atomic::{AtomicU64, Ordering};

/// The `draw step` phase of every buffer write: the harness reports it
/// by this name (`vapors_perf::report::PHASES`).
pub const WRITE_PHASE: &str = "write";

/// Bytes written to the GPU and draw calls issued. Structural: it counts
/// what was handed to wgpu, not how long the GPU took. wgpu's internal
/// row padding is not included.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct GpuWork {
    /// Bytes passed to `Queue::write_buffer`: instance, vertex and
    /// uniform data.
    pub buffer_bytes: u64,
    /// Bytes passed to `Queue::write_texture`: the glyph atlas page and
    /// image pixels, every mip level counted.
    pub texture_bytes: u64,
    /// `RenderPass::draw` and `RenderPass::draw_indexed` calls; one call
    /// may draw many quads.
    pub draw_calls: u64,
    /// `Queue::write_buffer` calls for a uniform value (a viewport, the
    /// text blend parameters); their bytes are in `buffer_bytes` too.
    pub uniform_writes: u64,
    /// `Device::create_bind_group` calls.
    pub bind_groups_made: u64,
    /// Kept-byte buffers grown: their live ranges copied into a buffer
    /// twice the size (`Retained`).
    pub retained_grown: u64,
    /// Kept-byte buffers compacted: their live ranges copied, packed,
    /// into a buffer of the same size (`Retained`).
    pub retained_compacted: u64,
}

impl GpuWork {
    /// Every byte written: buffers and textures.
    #[must_use]
    pub fn upload_bytes(&self) -> u64 {
        self.buffer_bytes + self.texture_bytes
    }
}

impl Add for GpuWork {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            buffer_bytes: self.buffer_bytes + other.buffer_bytes,
            texture_bytes: self.texture_bytes + other.texture_bytes,
            draw_calls: self.draw_calls + other.draw_calls,
            uniform_writes: self.uniform_writes + other.uniform_writes,
            bind_groups_made: self.bind_groups_made + other.bind_groups_made,
            retained_grown: self.retained_grown + other.retained_grown,
            retained_compacted: self.retained_compacted + other.retained_compacted,
        }
    }
}

impl AddAssign for GpuWork {
    fn add_assign(&mut self, other: Self) {
        *self = *self + other;
    }
}

/// Issues writes and draws and counts each as it is issued. Shared
/// through `&self`, so pipelines that draw through `&self` count too.
#[derive(Debug, Default)]
pub struct WorkCounter {
    buffer_bytes: AtomicU64,
    texture_bytes: AtomicU64,
    draw_calls: AtomicU64,
    uniform_writes: AtomicU64,
    bind_groups_made: AtomicU64,
    retained_grown: AtomicU64,
    retained_compacted: AtomicU64,
}

impl WorkCounter {
    /// `queue.write_buffer(buffer, offset, data)`, counting `data`.
    pub fn write_buffer(
        &self,
        queue: &wgpu::Queue,
        buffer: &wgpu::Buffer,
        offset: u64,
        data: &[u8],
    ) {
        let _write = tracing::trace_span!("draw step", phase = WRITE_PHASE).entered();
        queue.write_buffer(buffer, offset, data);
        self.buffer_bytes.fetch_add(len(data), Ordering::Relaxed);
    }

    /// [`WorkCounter::write_buffer`] of a uniform value, counting the
    /// write as well as `data`.
    pub fn write_uniform(
        &self,
        queue: &wgpu::Queue,
        buffer: &wgpu::Buffer,
        offset: u64,
        data: &[u8],
    ) {
        self.write_buffer(queue, buffer, offset, data);
        self.uniform_writes.fetch_add(1, Ordering::Relaxed);
    }

    /// `device.create_bind_group(descriptor)`, counting one bind group.
    pub fn create_bind_group(
        &self,
        device: &wgpu::Device,
        descriptor: &wgpu::BindGroupDescriptor<'_>,
    ) -> wgpu::BindGroup {
        let _bind = tracing::trace_span!("draw step", phase = "encode").entered();
        self.bind_groups_made.fetch_add(1, Ordering::Relaxed);
        device.create_bind_group(descriptor)
    }

    /// `queue.write_texture(..)` with these arguments, counting `data`.
    pub fn write_texture(
        &self,
        queue: &wgpu::Queue,
        texture: wgpu::TexelCopyTextureInfo<'_>,
        data: &[u8],
        layout: wgpu::TexelCopyBufferLayout,
        size: wgpu::Extent3d,
    ) {
        let _write = tracing::trace_span!("draw step", phase = WRITE_PHASE).entered();
        queue.write_texture(texture, data, layout, size);
        self.texture_bytes.fetch_add(len(data), Ordering::Relaxed);
    }

    /// Counts a kept-byte buffer moved: compacted, else grown.
    pub(crate) fn retained_moved(&self, compacted: bool) {
        let count = if compacted {
            &self.retained_compacted
        } else {
            &self.retained_grown
        };
        count.fetch_add(1, Ordering::Relaxed);
    }

    /// `pass.draw(vertices, instances)`, counting one call.
    pub fn draw(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        vertices: std::ops::Range<u32>,
        instances: std::ops::Range<u32>,
    ) {
        pass.draw(vertices, instances);
        self.draw_calls.fetch_add(1, Ordering::Relaxed);
    }

    /// `pass.draw_indexed(indices, base_vertex, instances)`, counting
    /// one call.
    pub fn draw_indexed(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        indices: std::ops::Range<u32>,
        base_vertex: i32,
        instances: std::ops::Range<u32>,
    ) {
        pass.draw_indexed(indices, base_vertex, instances);
        self.draw_calls.fetch_add(1, Ordering::Relaxed);
    }

    /// The work counted since the last take, and the count back to 0.
    pub fn take(&self) -> GpuWork {
        GpuWork {
            buffer_bytes: self.buffer_bytes.swap(0, Ordering::Relaxed),
            texture_bytes: self.texture_bytes.swap(0, Ordering::Relaxed),
            draw_calls: self.draw_calls.swap(0, Ordering::Relaxed),
            uniform_writes: self.uniform_writes.swap(0, Ordering::Relaxed),
            bind_groups_made: self.bind_groups_made.swap(0, Ordering::Relaxed),
            retained_grown: self.retained_grown.swap(0, Ordering::Relaxed),
            retained_compacted: self.retained_compacted.swap(0, Ordering::Relaxed),
        }
    }
}

fn len(data: &[u8]) -> u64 {
    data.len() as u64
}
