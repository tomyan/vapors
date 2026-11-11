//! A quad pipeline's targets for the `FrameEncoder` tests: square
//! textures, one-pixel and larger quads in a colour of their own.

// each test crate including this uses a part of it
#![allow(dead_code)]

use vapors_gpu_harness::Gpu;
use vapors_renderer::{FrameEncoder, Target, clear};
use vapors_scene::{Color, Quad, Rect, Size};

pub const BACKGROUND: [u8; 4] = [40, 40, 40, 255];

/// A square `side` texture cleared to [`BACKGROUND`], and its view.
pub fn square(gpu: &Gpu, side: u32) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = clear(&gpu.device, &gpu.queue, side, side, rgba(BACKGROUND));
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}

/// A target on `view`, a square `side` across, recording into `encoder`.
pub fn target<'a>(
    gpu: &'a Gpu,
    encoder: &'a FrameEncoder,
    view: &'a wgpu::TextureView,
    side: u32,
) -> Target<'a> {
    Target {
        device: &gpu.device,
        queue: &gpu.queue,
        encoder,
        view,
        viewport: Size {
            width: side,
            height: side,
        },
        scissor: None,
    }
}

/// A quad filling `left..right` by `top..bottom` in `colour`.
pub fn quad([left, top, right, bottom]: [f32; 4], colour: [u8; 4]) -> Quad {
    Quad::fill(Rect::new(left, top, right, bottom), rgba(colour))
}

pub fn rgba([red, green, blue, alpha]: [u8; 4]) -> Color {
    Color::from_rgba8(red, green, blue, alpha)
}

/// The RGBA of pixel `(x, y)` in `pixels`, rows `side` wide.
pub fn pixel(pixels: &[u8], side: u32, x: u32, y: u32) -> [u8; 4] {
    let at = ((y * side + x) * 4) as usize;
    pixels[at..at + 4].try_into().expect("four bytes")
}

/// The queue's next submission's index, less one. wgpu keeps the
/// number private; its `Debug` form, `SubmissionIndex { index: N }`,
/// is the only reading, so any other form fails here, naming it.
pub fn last_submission(gpu: &Gpu) -> u64 {
    let index = format!("{:?}", gpu.queue.submit([]));
    let digits = index
        .strip_prefix("SubmissionIndex { index: ")
        .and_then(|rest| rest.strip_suffix(" }"))
        .unwrap_or_else(|| panic!("unrecognised SubmissionIndex form: {index}"));
    digits.parse().expect("a submission number")
}
