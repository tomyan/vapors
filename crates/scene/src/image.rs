//! An image's pixels, as the element tree and the canvas take them.

use crate::Size;

/// Tightly packed 8-bit RGBA pixels with their size.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RgbaImage {
    /// Width and height in pixels.
    pub size: Size,
    /// `size.width * size.height * 4` bytes, rows from the top.
    pub rgba: Vec<u8>,
}
