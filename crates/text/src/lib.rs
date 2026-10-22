//! Text: Parley layout over the bundled Geist fonts, CPU glyph
//! rasterisation into an atlas, gamma-correct drawing through wgpu.
//!
//! GPU tests for this crate live in `tests/gpu_*.rs`, are named `gpu_*`,
//! and run in the `gpu` tier (`cargo xtask test --tier gpu`).

mod atlas;
mod atlas_page;
mod font_system;
pub mod fonts;
mod layout;
mod pen;
mod raster;
mod style;

pub use atlas::{AtlasConfig, AtlasError, AtlasGlyph, GlyphAtlas, PageKind};
pub use atlas_page::AtlasPage;
pub use font_system::FontSystem;
pub use layout::{Align, Glyph, GlyphRun, Layout, LayoutOptions, LineInfo, PlacedGlyph};
pub use parley;
pub use raster::{
    Coverage, GlyphRequest, Hinting, RasterError, Rasterizer, SUBPIXEL_STEPS, subpixel_of,
};
pub use style::{Family, Span, Style};
