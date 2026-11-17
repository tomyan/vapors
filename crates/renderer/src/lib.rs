//! Renderer: turns a scene into pixels with wgpu.
//!
//! GPU tests for this crate live in `tests/gpu_*.rs`, are named `gpu_*`,
//! and run in the `gpu` tier (`cargo xtask test --tier gpu`).

mod arena_full;
mod clear;
mod closed_form_shape;
mod closed_forms;
mod closed_forms_3d;
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
mod instanced_bytes;
mod kept_bind_group;
mod kept_range;
mod mesh_entries;
mod mesh_shape;
mod meshes;
mod meshes_3d;
mod meshes_bytes;
mod meshes_setup;
mod mips;
mod open_pass;
pub mod pipelines;
pub mod projective;
mod quad;
mod quads;
mod quads_setup;
mod readback;
mod retained;
mod shadows;
mod solid;
mod table_binding;
mod target;
mod transform_entries;
mod transform_table;
mod uniform_slots;
mod upload;
mod word_hash;

#[cfg(test)]
mod gpu_uniform_slots_tests;
mod work;

pub use arena_full::{ArenaFull, Holding};
pub use clear::{clear, clear_target, clear_view};
pub use closed_form_shape::{ClosedFormError, ClosedFormShape};
pub use closed_forms::ClosedFormPipeline;
pub use closed_forms_3d::{CLOSED_FORM_STRIDE, CLOSED_FORMS_3D_WGSL, ProjectiveClosedForms};
pub use encoder_budget::EncoderBudget;
pub use frame_arena::{FrameArena, Placed};
pub use frame_clock::FrameClock;
pub use frame_encoder::FrameEncoder;
pub use gpu_time::{GpuMissing, GpuReading, GpuResolution, GpuTime};
pub use gpu_timer::{DEFAULT_PAIRS, GpuTimer, TIMER_FEATURES};
pub use image::ImagePipeline;
pub use image_binding::{Sampling, TextureBinding};
pub use instanced_bytes::assert_rows;
pub use kept_bind_group::{Generations, KeptBindGroup};
pub use kept_range::KeptRange;
pub use mesh_entries::{MeshEntries, MeshEntry, mesh_entry_bytes};
pub use mesh_shape::MeshShape;
pub use meshes::MeshPipeline;
pub use meshes_3d::ProjectiveMeshes;
pub use mips::{upload_rgba8_mipmapped, upload_rgba8_mipmapped_counted};
pub use projective::{Bound, ScenePassFormats};
pub use quads::QuadPipeline;
pub use readback::read_rgba8;
pub use retained::Retained;
pub use shadows::ShadowPipeline;
pub use solid::SolidPipeline;
pub use target::Target;
pub use transform_entries::{IDENTITY, Matrix, translation};
pub use transform_table::TransformTable;
pub use uniform_slots::UniformSlots;
pub use upload::upload_rgba8;
pub use work::{GpuWork, WRITE_PHASE, WorkCounter};
