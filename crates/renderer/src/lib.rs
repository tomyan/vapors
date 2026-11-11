//! Renderer: turns a scene into pixels with wgpu.
//!
//! GPU tests for this crate live in `tests/gpu_*.rs`, are named `gpu_*`,
//! and run in the `gpu` tier (`cargo xtask test --tier gpu`).

mod arena_full;
mod clear;
mod frame_arena;
mod frame_encoder;
mod image;
mod image_setup;
mod instanced;
mod mips;
mod open_pass;
mod quad;
mod quads;
mod quads_setup;
mod readback;
mod shadows;
mod solid;
mod target;
mod upload;
mod work;

pub use arena_full::ArenaFull;
pub use clear::{clear, clear_target, clear_view};
pub use frame_arena::{FrameArena, Placed};
pub use frame_encoder::FrameEncoder;
pub use image::ImagePipeline;
pub use mips::{upload_rgba8_mipmapped, upload_rgba8_mipmapped_counted};
pub use quads::QuadPipeline;
pub use readback::read_rgba8;
pub use shadows::ShadowPipeline;
pub use solid::SolidPipeline;
pub use target::Target;
pub use upload::upload_rgba8;
pub use work::{GpuWork, WorkCounter};
