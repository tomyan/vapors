//! The comparison page: a fixed set of paragraphs rendered by this crate
//! and by headless Chromium from the same fonts, sizes and colours, so
//! text quality can be judged side by side and the gamma and contrast
//! parameters tuned. `docs/text.md` records the outcome.
//!
//! The page spec and HTML compile everywhere; running Chromium and
//! reading PNGs are native only. Both 1x and 2x are rendered: the brief's
//! requirement 1 names both displays.

#[cfg(not(target_family = "wasm"))]
mod chromium;
mod composite;
mod html;
mod measure;
mod page;
#[cfg(not(target_family = "wasm"))]
mod png_file;
mod spec;

#[cfg(not(target_family = "wasm"))]
pub use chromium::{Chromium, ChromiumError};
pub use composite::{GAP, compose, row_width};
pub use html::{FONT_FILES, html};
pub use measure::{INK_BAND, Rgba, ink_within_band, mean_ink};
pub use page::{render_labels, render_page};
#[cfg(not(target_family = "wasm"))]
pub use png_file::{read_png, write_png};
pub use spec::{
    Block, MEASURED_BLOCK, PAGE_HEIGHT, PAGE_WIDTH, SCHEMES, Scheme, block_rect, blocks,
    body_measure_rect,
};
