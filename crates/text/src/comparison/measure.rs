//! Numbers from the renderings: how much ink a region has, and whether
//! ours is within the band of Chromium's. A loose guard against the
//! renderer going blank or bold, not a quality judge.

/// Tightly packed 8-bit RGBA pixels.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rgba {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// `width * height * 4` bytes, rows from the top.
    pub data: Vec<u8>,
}

impl Rgba {
    /// The pixel at `x`, `y`.
    ///
    /// # Panics
    ///
    /// If the point is outside the image; a caller bug.
    #[must_use]
    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        assert!(
            x < self.width && y < self.height,
            "({x}, {y}) outside {}x{}",
            self.width,
            self.height
        );
        let start = ((y * self.width + x) * 4) as usize;
        self.data[start..start + 4].try_into().expect("four bytes")
    }
}

/// Ratio of our mean ink to Chromium's that passes: between these two.
/// Chosen from the measured ratio on 2026-10-22 (see `docs/text.md`) with
/// room for Chromium's fontconfig hinting differing between machines;
/// outside it the renderer has gone blank, bold or misplaced.
pub const INK_BAND: [f32; 2] = [0.7, 1.4];

/// Mean distance from `background` over `rect` (`[x, y, width, height]`),
/// 0 for an empty region, 1 for solid ink of the opposite colour, by the
/// channel that moved most.
///
/// # Panics
///
/// If `rect` leaves the image; a caller bug.
#[must_use]
pub fn mean_ink(image: &Rgba, rect: [u32; 4], background: [u8; 4]) -> f32 {
    let [x0, y0, w, h] = rect;
    assert!(
        x0 + w <= image.width && y0 + h <= image.height,
        "{rect:?} leaves the image"
    );
    let mut total = 0u64;
    for y in y0..y0 + h {
        for x in x0..x0 + w {
            let pixel = image.pixel(x, y);
            total += u64::from(
                pixel
                    .iter()
                    .zip(background)
                    .map(|(p, b)| p.abs_diff(b))
                    .max()
                    .unwrap_or(0),
            );
        }
    }
    #[allow(clippy::cast_precision_loss)] // page-sized regions
    let mean = total as f64 / (u64::from(w) * u64::from(h) * 255) as f64;
    #[allow(clippy::cast_possible_truncation)] // 0..=1
    let mean = mean as f32;
    mean
}

/// True when `ours / theirs` is inside [`INK_BAND`].
#[must_use]
pub fn ink_within_band(ours: f32, theirs: f32) -> bool {
    theirs > 0.0 && (INK_BAND[0]..=INK_BAND[1]).contains(&(ours / theirs))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image(width: u32, height: u32, fill: [u8; 4]) -> Rgba {
        Rgba {
            width,
            height,
            data: fill.repeat((width * height) as usize),
        }
    }

    #[test]
    fn mean_ink_is_zero_on_the_background_and_one_on_solid_opposite() {
        // Given
        let background = [23, 27, 31, 255];
        let empty = image(4, 2, background);
        let mut half = image(4, 2, background);
        half.data[0..4].copy_from_slice(&[255, 255, 255, 255]);

        // When / Then
        assert_eq!(mean_ink(&empty, [0, 0, 4, 2], background), 0.0);
        let one_of_eight = mean_ink(&half, [0, 0, 4, 2], background);
        assert!(
            (one_of_eight - (232.0 / 255.0) / 8.0).abs() < 1e-6,
            "{one_of_eight}"
        );
        assert_eq!(
            mean_ink(&half, [1, 0, 3, 2], background),
            0.0,
            "region excludes the pixel"
        );
    }

    #[test]
    fn the_band_accepts_near_ratios_and_refuses_blank_or_bold() {
        assert!(ink_within_band(0.10, 0.10));
        assert!(ink_within_band(0.08, 0.10));
        assert!(!ink_within_band(0.0, 0.10), "blank");
        assert!(!ink_within_band(0.20, 0.10), "twice as heavy");
        assert!(!ink_within_band(0.10, 0.0), "nothing to compare with");
    }
}
