//! Text: Parley layout over the bundled Geist fonts, CPU glyph
//! rasterisation into an atlas, gamma-correct drawing through wgpu.
//!
//! GPU tests for this crate live in `tests/gpu_*.rs`, are named `gpu_*`,
//! and run in the `gpu` tier (`cargo xtask test --tier gpu`).

mod atlas;
mod atlas_evict;
mod atlas_page;
mod atlas_types;
pub mod comparison;
mod draw;
mod font_system;
pub mod fonts;
#[cfg(test)]
mod italic_tests;
mod layout;
mod layout_ranges;
mod layout_runs;
mod layout_types;
mod line_metrics;
mod pen;
mod pipeline;
mod pipeline_gpu;
mod quads;
mod raster;
mod raster_types;
mod style;

pub use atlas::{AtlasConfig, AtlasError, AtlasGlyph, GlyphAtlas, PageKind};
pub use atlas_page::AtlasPage;
pub use draw::RenderTarget;
pub use font_system::FontSystem;
pub use layout::{Align, Glyph, GlyphRun, Layout, LayoutOptions, LineInfo, PlacedGlyph};
pub use layout_ranges::Fragment;
pub use line_metrics::content_area;
pub use parley;
pub use pipeline::{BlendParams, TextPipeline};
pub use raster::{
    Coverage, GlyphRequest, Hinting, RasterError, Rasterizer, SUBPIXEL_STEPS, subpixel_of,
};
pub use style::{Family, LineHeight, Span, Style};

/// How far a synthesised italic moves x right per pixel up: Skia's fake
/// italic (`SK_Scalar1 / 4`), so a face with no italic slants as it does
/// in Chromium. Parley reports that a run needs one; this is the amount.
pub const SYNTHETIC_SLANT: f32 = 0.25;
