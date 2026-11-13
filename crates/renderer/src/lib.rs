//! Renderer: turns a scene into pixels with wgpu.
//!
//! GPU tests for this crate live in `tests/gpu_*.rs`, are named `gpu_*`,
//! and run in the `gpu` tier (`cargo xtask test --tier gpu`).

mod arena_full;
mod clear;
mod command_encoder;
mod encoder_budget;
mod encoder_turn;
mod frame_arena;
mod frame_clock;
mod frame_encoder;
mod free_ranges;
mod gpu_time;
mod gpu_timer;
mod gpu_timer_slot;
mod image;
mod image_binding;
mod image_setup;
mod instanced;
mod kept_bind_group;
mod kept_range;
mod mips;
mod open_pass;
mod quad;
mod quads;
mod quads_setup;
mod readback;
mod retained;
mod shadows;
mod solid;
mod target;
mod uniform_slots;
mod upload;
mod work;

pub use arena_full::{ArenaFull, Holding};
pub use clear::{clear, clear_target, clear_view};
pub use encoder_budget::EncoderBudget;
pub use frame_arena::{FrameArena, Placed};
pub use frame_clock::FrameClock;
pub use frame_encoder::FrameEncoder;
pub use gpu_time::{GpuMissing, GpuReading, GpuResolution, GpuTime};
pub use gpu_timer::{DEFAULT_PAIRS, GpuTimer, TIMER_FEATURES};
pub use image::ImagePipeline;
pub use image_binding::{Sampling, TextureBinding};
pub use kept_bind_group::KeptBindGroup;
pub use kept_range::KeptRange;
pub use mips::{upload_rgba8_mipmapped, upload_rgba8_mipmapped_counted};
pub use quads::QuadPipeline;
pub use readback::read_rgba8;
pub use retained::Retained;
pub use shadows::ShadowPipeline;
pub use solid::SolidPipeline;
pub use target::Target;
pub use uniform_slots::UniformSlots;
pub use upload::upload_rgba8;
pub use work::{GpuWork, WRITE_PHASE, WorkCounter};
