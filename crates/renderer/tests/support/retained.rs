//! Quads kept by a quad pipeline (`retained.rs`) on a small square, for
//! the `gpu_retained` tests.

// each test crate including this uses a part of it
#![allow(dead_code)]

use vapors_gpu_harness::{Gpu, read_rgba8};
use vapors_renderer::{FrameEncoder, KeptRange, QuadPipeline, Shift};

use super::frame::{pixel, quad, square, target};

pub const SIDE: u32 = 4;
/// A quad's instance bytes.
pub const QUAD_BYTES: u64 = 128;
/// A kept buffer's first size (`FIRST_SIZE` in `retained.rs`): 512 quads.
pub const FIRST_SIZE: u64 = 64 * 1024;
pub const RED: [u8; 4] = [255, 0, 0, 255];
pub const GREEN: [u8; 4] = [0, 255, 0, 255];
pub const BLUE: [u8; 4] = [0, 0, 255, 255];

/// A square to draw on and a quad pipeline.
pub struct Frame {
    pub gpu: Gpu,
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub pipeline: QuadPipeline,
}

impl Frame {
    pub fn new(gpu: Gpu) -> Self {
        let (texture, view) = square(&gpu, SIDE);
        let pipeline = QuadPipeline::new(&gpu.device, texture.format());
        Self {
            gpu,
            texture,
            view,
            pipeline,
        }
    }

    /// `count` quads in `colour` over pixel `(x, 0)`, kept.
    pub fn keep(
        &mut self,
        encoder: &FrameEncoder,
        (count, x, colour): (usize, f32, [u8; 4]),
    ) -> Option<KeptRange> {
        let quads = vec![quad([x, 0.0, x + 1.0, 1.0], colour); count];
        let at = target(&self.gpu, encoder, &self.view, SIDE);
        self.pipeline.keep(&at, &QuadPipeline::bytes(&quads))
    }

    pub fn draw(&mut self, encoder: &FrameEncoder, kept: &KeptRange) {
        self.draw_shifted(encoder, kept, Shift::NONE);
    }

    /// Draws `kept` moved by `shift`, device pixels.
    pub fn draw_shifted(&mut self, encoder: &FrameEncoder, kept: &KeptRange, shift: Shift) {
        let at = target(&self.gpu, encoder, &self.view, SIDE);
        self.pipeline.draw_kept(&at, kept, shift);
    }

    /// The colours of the top row's first three pixels.
    pub fn row(&self) -> [[u8; 4]; 3] {
        let rgba = read_rgba8(&self.gpu.device, &self.gpu.queue, &self.texture);
        [0, 1, 2].map(|x| pixel(&rgba, SIDE, x, 0))
    }

    pub fn encoder(&self) -> FrameEncoder {
        FrameEncoder::new(&self.gpu.device)
    }
}

/// Kept, or a panic naming `what`.
pub fn kept(range: Option<KeptRange>, what: &str) -> KeptRange {
    range.unwrap_or_else(|| panic!("{what} refused"))
}
