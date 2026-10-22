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
mod tests {
    use super::*;

    #[test]
    fn allocations_run_along_a_shelf_then_start_the_next() {
        // Given: a 10 px page, glyphs 4x4 need 5x5 with padding
        let mut page = AtlasPage::new(10, 1);

        // When
        let first = page.allocate(4, 4);
        let second = page.allocate(4, 4);
        let third = page.allocate(4, 4);
        let fifth = {
            page.allocate(4, 4);
            page.allocate(4, 4)
        };

        // Then
        assert_eq!(first, Some([0, 0]));
        assert_eq!(second, Some([5, 0]));
        assert_eq!(third, Some([0, 5]));
        assert_eq!(fifth, None, "page is full");
    }

    /// Shelf packing is order dependent: the reviewer's shape. Four 4x4
    /// and two 4x1 glyphs (5x5 and 5x2 with padding) in a 13 px page need
    /// three 5-high shelves when interleaved (15 px) and two 5-high plus
    /// one 2-high when tallest first (12 px).
    #[test]
    fn tallest_first_fits_where_interleaved_order_does_not() {
        // Given
        let interleaved = [(4, 1), (4, 4), (4, 1), (4, 4), (4, 4), (4, 4)];
        let tallest_first = [(4, 4), (4, 4), (4, 4), (4, 4), (4, 1), (4, 1)];

        // When
        let mut page = AtlasPage::new(13, 1);
        let in_order: Vec<_> = interleaved
            .iter()
            .map(|&(w, h)| page.allocate(w, h))
            .collect();
        page.clear();
        let sorted: Vec<_> = tallest_first
            .iter()
            .map(|&(w, h)| page.allocate(w, h))
            .collect();

        // Then
        assert!(in_order.iter().any(Option::is_none), "{in_order:?}");
        assert!(sorted.iter().all(Option::is_some), "{sorted:?}");
    }

    #[test]
    fn a_glyph_larger_than_the_page_is_refused() {
        // Given
        let mut page = AtlasPage::new(8, 1);

        // When / Then
        assert!(!page.fits(8, 2));
        assert_eq!(page.allocate(8, 2), None);
        assert_eq!(page.allocate(2, 8), None);
        assert!(page.fits(7, 7));
    }

    #[test]
    fn blit_writes_rows_at_the_origin_and_marks_the_page_dirty() {
        // Given
        let mut page = AtlasPage::new(4, 1);
        page.mark_clean();

        // When
        page.blit([1, 2], 2, 2, &[1, 2, 3, 4]);

        // Then
        assert!(page.dirty());
        assert_eq!(&page.pixels()[8..12], &[0, 1, 2, 0]);
        assert_eq!(&page.pixels()[12..16], &[0, 3, 4, 0]);
    }
}
