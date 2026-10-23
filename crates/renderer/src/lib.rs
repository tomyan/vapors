//! Renderer: turns a scene into pixels with wgpu.
//!
//! GPU tests for this crate live in `tests/gpu_*.rs`, are named `gpu_*`,
//! and run in the `gpu` tier (`cargo xtask test --tier gpu`).

mod clear;
mod image;
mod instanced;
mod quad;
mod quads;
mod quads_setup;
mod readback;
mod shadows;
mod solid;
mod upload;

pub use clear::{clear, clear_view};
pub use image::ImagePipeline;
pub use instanced::Target;
pub use quads::QuadPipeline;
pub use readback::read_rgba8;
pub use shadows::ShadowPipeline;
pub use solid::SolidPipeline;
pub use upload::upload_rgba8;
