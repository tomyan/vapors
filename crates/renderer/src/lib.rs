//! Renderer: turns a scene into pixels with wgpu.
//!
//! GPU tests for this crate live in `tests/gpu_*.rs` and run in the
//! `gpu` tier (`cargo xtask test --tier gpu`).

mod clear;

pub use clear::clear;
