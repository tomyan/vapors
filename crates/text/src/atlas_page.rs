//! One texture page of the atlas: a shelf packer over a byte buffer.
//! Glyphs are packed left to right in rows (shelves) whose height is the
//! first glyph's; a glyph that does not fit the row starts a new one.

/// A page of `bytes_per_pixel` pixels, `size` square.
pub struct AtlasPage {
    size: u32,
    bytes_per_pixel: u32,
    pixels: Vec<u8>,
    shelf_x: u32,
    shelf_y: u32,
    shelf_height: u32,
    dirty: bool,
}

/// Gap kept around every glyph so nearest sampling never reads a neighbour.
const PADDING: u32 = 1;

impl AtlasPage {
    /// An empty page.
    #[must_use]
    pub fn new(size: u32, bytes_per_pixel: u32) -> Self {
        Self {
            size,
            bytes_per_pixel,
            pixels: vec![0; (size * size * bytes_per_pixel) as usize],
            shelf_x: 0,
            shelf_y: 0,
            shelf_height: 0,
            dirty: true,
        }
    }

    /// Side length in pixels.
    #[must_use]
    pub fn size(&self) -> u32 {
        self.size
    }

    /// The pixels, rows from the top, `size * size * bytes_per_pixel`.
    #[must_use]
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// True when the pixels changed since [`AtlasPage::mark_clean`].
    #[must_use]
    pub fn dirty(&self) -> bool {
        self.dirty
    }

    /// The caller uploaded the pixels.
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    /// True when a `width` by `height` glyph could ever be placed in an
    /// empty page.
    #[must_use]
    pub fn fits(&self, width: u32, height: u32) -> bool {
        width + PADDING <= self.size && height + PADDING <= self.size
    }

    /// Reserve `width` by `height` pixels; the top-left of the reserved
    /// area, or `None` when the page is full.
    pub fn allocate(&mut self, width: u32, height: u32) -> Option<[u32; 2]> {
        let (padded_w, padded_h) = (width + PADDING, height + PADDING);
        if !self.fits(width, height) {
            return None;
        }
        if self.shelf_x + padded_w > self.size {
            self.shelf_y += self.shelf_height;
            self.shelf_x = 0;
            self.shelf_height = 0;
        }
        if self.shelf_y + padded_h > self.size {
            return None;
        }
        let origin = [self.shelf_x, self.shelf_y];
        self.shelf_x += padded_w;
        self.shelf_height = self.shelf_height.max(padded_h);
        Some(origin)
    }

    /// Copy `data` (`width * height * bytes_per_pixel`) to `origin`.
    ///
    /// # Panics
    ///
    /// If `data` is not that length or the rectangle leaves the page; both
    /// are caller bugs and the message says which.
    pub fn blit(&mut self, origin: [u32; 2], width: u32, height: u32, data: &[u8]) {
        let bpp = self.bytes_per_pixel as usize;
        assert_eq!(
            data.len(),
            (width * height) as usize * bpp,
            "blit data length"
        );
        assert!(
            origin[0] + width <= self.size && origin[1] + height <= self.size,
            "blit of {width}x{height} at {origin:?} leaves a {} page",
            self.size
        );
        let row_bytes = width as usize * bpp;
        for row in 0..height as usize {
            let start =
                ((origin[1] as usize + row) * self.size as usize + origin[0] as usize) * bpp;
            self.pixels[start..start + row_bytes]
                .copy_from_slice(&data[row * row_bytes..(row + 1) * row_bytes]);
        }
        self.dirty = true;
    }

    /// Forget every allocation and zero the pixels.
    pub fn clear(&mut self) {
        self.pixels.fill(0);
        self.shelf_x = 0;
        self.shelf_y = 0;
        self.shelf_height = 0;
        self.dirty = true;
    }
}

#[cfg(test)]
#[path = "atlas_page_tests.rs"]
mod tests;
