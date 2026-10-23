//! Holding a screenshot to a section's checks.

use crate::parse::Check;

/// An RGBA8 image, rows top to bottom.
pub struct Image<'a> {
    /// Width in device pixels.
    pub width: u32,
    /// Height in device pixels.
    pub height: u32,
    /// Device pixels per CSS pixel.
    pub scale: f32,
    /// `width * height * 4` bytes.
    pub rgba: &'a [u8],
}

impl Image<'_> {
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )] // small screen coordinates
    fn at(&self, x: u32, y: u32) -> Option<[u8; 3]> {
        let (x, y) = (
            (x as f32 * self.scale) as u32,
            (y as f32 * self.scale) as u32,
        );
        if x >= self.width || y >= self.height {
            return None;
        }
        let start = ((y * self.width + x) * 4) as usize;
        Some([self.rgba[start], self.rgba[start + 1], self.rgba[start + 2]])
    }
}

/// The checks that fail on `image`, each as a line naming what was
/// wanted and found.
#[must_use]
pub fn failures(checks: &[Check], image: &Image<'_>) -> Vec<String> {
    checks
        .iter()
        .filter_map(|check| failure(check, image))
        .collect()
}

fn failure(check: &Check, image: &Image<'_>) -> Option<String> {
    match check {
        Check::Pixel { x, y, rgb } => match image.at(*x, *y) {
            Some(found) if close(found, *rgb) => None,
            found => Some(format!(
                "pixel {x},{y}: wanted {}, found {}",
                hex(*rgb),
                found.map_or("nothing (outside the image)".to_owned(), hex)
            )),
        },
        Check::Ink { from, to, rgb } => {
            let inked = (from[1]..to[1])
                .any(|y| (from[0]..to[0]).any(|x| image.at(x, y).is_some_and(|p| !close(p, *rgb))));
            (!inked).then(|| {
                format!(
                    "ink {from:?} to {to:?}: every pixel is {}; nothing was drawn there",
                    hex(*rgb)
                )
            })
        }
    }
}

fn close(a: [u8; 3], b: [u8; 3]) -> bool {
    a.iter().zip(b).all(|(p, q)| p.abs_diff(q) <= 1)
}

fn hex(rgb: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2])
}
