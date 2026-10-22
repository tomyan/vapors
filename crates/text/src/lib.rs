//! Text: Parley layout over the bundled Geist fonts, CPU glyph
//! rasterisation into an atlas, gamma-correct drawing through wgpu.
//!
//! GPU tests for this crate live in `tests/gpu_*.rs`, are named `gpu_*`,
//! and run in the `gpu` tier (`cargo xtask test --tier gpu`).

pub mod fonts;
