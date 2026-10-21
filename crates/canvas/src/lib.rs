//! Canvas: an image shown in a viewport. This is the seed of the
//! framework's canvas surface element. It owns the view transform,
//! answers input (scroll zooms about the pointer, `0` fits, `1` is actual
//! size, `+` and `-` zoom about the centre) and draws through the
//! renderer. The pixels it shows belong to whoever uploaded them.

mod image_view;

pub use image_view::{ImageView, RgbaImage};
