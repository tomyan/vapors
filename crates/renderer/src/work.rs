//! GPU work counted on the CPU where it is issued: bytes handed to the
//! queue's writes and draw calls recorded, with no readback and no wait.

use std::ops::{Add, AddAssign};
use std::sync::atomic::{AtomicU64, Ordering};

/// Bytes written to the GPU and draw calls issued. Structural: it counts
/// what was handed to wgpu, not how long the GPU took.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct GpuWork {
    /// Bytes passed to `Queue::write_buffer`: instance, vertex and
    /// uniform data.
    pub buffer_bytes: u64,
    /// Bytes passed to `Queue::write_texture`: the glyph atlas page and
    /// image pixels, every mip level counted.
    pub texture_bytes: u64,
    /// `RenderPass::draw` calls; one call may draw many quads.
    pub draw_calls: u64,
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
        queue.write_buffer(buffer, offset, data);
        self.buffer_bytes.fetch_add(len(data), Ordering::Relaxed);
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
        queue.write_texture(texture, data, layout, size);
        self.texture_bytes.fetch_add(len(data), Ordering::Relaxed);
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

    /// The work counted since the last take, and the count back to 0.
    pub fn take(&self) -> GpuWork {
        GpuWork {
            buffer_bytes: self.buffer_bytes.swap(0, Ordering::Relaxed),
            texture_bytes: self.texture_bytes.swap(0, Ordering::Relaxed),
            draw_calls: self.draw_calls.swap(0, Ordering::Relaxed),
        }
    }
}

fn len(data: &[u8]) -> u64 {
    data.len() as u64
}
