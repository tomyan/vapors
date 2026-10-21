//! Renderer: turns a scene into pixels with wgpu.
//!
//! GPU tests for this crate live in `tests/gpu_*.rs` and run in the
//! `gpu` tier (`cargo xtask test --tier gpu`).

mod clear;
mod image;
mod readback;
mod upload;

pub use clear::{clear, clear_view};
pub use image::ImagePipeline;
pub use readback::read_rgba8;
pub use upload::upload_rgba8;
