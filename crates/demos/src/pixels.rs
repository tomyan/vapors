//! Pixel checks for a demo's screenshot. One file per demo,
//! `demos/<name>.pixels`, read by the native screenshot (the demo
//! example) and the web screenshot (the page-contract script, through
//! the `check_pixels` binary), so the two cannot drift apart.
//!
//! ```text
//! pixel X Y #rrggbb            exact, within 1 per channel
//! ink X0 Y0 X1 Y1 #rrggbb      some pixel in the box is not that colour
//! ```
//!
//! Coordinates are CSS pixels; `#` after the fields starts a comment.

/// One check.
#[derive(Clone, Debug, PartialEq)]
pub enum Check {
    /// The pixel at (x, y) is this colour.
    Pixel {
        /// CSS pixels from the left.
        x: u32,
        /// CSS pixels from the top.
        y: u32,
        /// The colour.
        rgb: [u8; 3],
    },
    /// Some pixel in the box differs from this colour: something was drawn.
    Ink {
        /// Top-left corner, included.
        from: [u32; 2],
        /// Bottom-right corner, excluded.
        to: [u32; 2],
        /// The colour the box would be with nothing drawn on it.
        rgb: [u8; 3],
    },
}

/// The checks in a `.pixels` file.
///
/// # Errors
/// A line that is not one of the two shapes, naming the line.
pub fn parse(name: &str, text: &str) -> Result<Vec<Check>, String> {
    text.lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty() && !line.trim_start().starts_with('#'))
        .map(|(index, line)| {
            parse_line(line).map_err(|e| format!("{name}.pixels:{}: {e}: {line}", index + 1))
        })
        .collect()
}

fn parse_line(line: &str) -> Result<Check, String> {
    let fields: Vec<&str> = line.split_whitespace().collect();
    match fields.as_slice() {
        ["pixel", x, y, rgb, ..] => Ok(Check::Pixel {
            x: number(x)?,
            y: number(y)?,
            rgb: colour(rgb)?,
        }),
        ["ink", x0, y0, x1, y1, rgb, ..] => Ok(Check::Ink {
            from: [number(x0)?, number(y0)?],
            to: [number(x1)?, number(y1)?],
            rgb: colour(rgb)?,
        }),
        _ => Err("expected `pixel X Y #rrggbb` or `ink X0 Y0 X1 Y1 #rrggbb`".to_owned()),
    }
}

fn number(text: &str) -> Result<u32, String> {
    text.parse()
        .map_err(|_| format!("`{text}` is not a whole number of pixels"))
}

fn colour(text: &str) -> Result<[u8; 3], String> {
    let hex = text
        .strip_prefix('#')
        .filter(|h| h.len() == 6)
        .ok_or(format!("`{text}` is not #rrggbb"))?;
    let channel = |i: usize| {
        u8::from_str_radix(&hex[i..i + 2], 16).map_err(|_| format!("`{text}` is not #rrggbb"))
    };
    Ok([channel(0)?, channel(2)?, channel(4)?])
}

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

#[cfg(test)]
#[path = "pixels_tests.rs"]
mod tests;
