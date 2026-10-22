//! Canvas: an image shown in a viewport. This is the seed of the
//! framework's canvas surface element. It owns the view transform,
//! answers input (scroll zooms about the pointer, `0` fits, `1` is actual
//! size, `+` and `-` zoom about the centre) and draws through the
//! renderer. The pixels it shows belong to whoever uploaded them.

mod image_app;
mod image_view;
mod image_view_draw;
mod label;
#[cfg(test)]
mod pinch_tests;
mod png;

pub use image_app::ImageApp;
pub use image_view::{ImageView, RgbaImage};
pub use label::{INK, LabelDraw, STRIP_COLOR, STRIP_HEIGHT, label_text, strip_height};
pub use png::decode_png;
