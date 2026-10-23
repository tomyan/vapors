//! The pixel rules a paint fixture is held to against Chromium, applied
//! to two RGBA images of one size:
//!
//! 1. Where Chromium's pixel and its eight neighbours are one colour
//!    (within `FLAT_NEIGHBOURS`), ours is within `FLAT_TOLERANCE`.
//! 2. Every pixel is within `EDGE_BOUND`.
//! 3. In each fixture element's region (its border box in device
//!    pixels, one pixel larger all round), at most `BUDGET` pixels are
//!    strongly off: more than `STRONG_FLOOR` apart and more than half
//!    the local contrast (the larger channel range of either image's
//!    5x5 neighbourhood).
//!
//! Rule 3 is what catches a shape error the first two pass. A radius of
//! 6 where Chromium has 8 changes a few corner pixels from covered to
//! uncovered: each differs by about the whole local contrast, which is
//! small for a white card on a light page (22 levels) but large next to
//! that contrast. Anti-aliasing that differs from Skia's moves a pixel
//! by a fraction of it (a quarter, at most, here). Measured on
//! `paint_boxes` (Chromium 155, 2026-10-23, Intel and lavapipe): ours
//! has no strongly-off pixel in any region at 1x or 2x; Chromium with
//! the card's radius at 6 has 13 in the card at 1x and 90 at 2x, the
//! unshadowed box's radius at 6 has 14 and 90, and the card moved 1 px
//! has 129 and 438. Over the whole image, by count or by magnitude
//! alone, those pixels were lost among anti-aliasing differences.

/// How alike Chromium's neighbourhood must be to count as flat.
pub const FLAT_NEIGHBOURS: u8 = 1;
/// How far ours may be from Chromium in a flat neighbourhood.
pub const FLAT_TOLERANCE: u8 = 2;
/// How far any pixel may be: 86 measured at a corner join, plus room.
pub const EDGE_BOUND: u8 = 96;
/// A difference this small is never strongly off, however flat.
pub const STRONG_FLOOR: u8 = 3;
/// Strongly-off pixels an element's region may hold.
pub const BUDGET: usize = 2;

/// An RGBA8 image, rows top to bottom.
pub struct Image<'a> {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// `width * height * 4` bytes.
    pub rgba: &'a [u8],
}

impl Image<'_> {
    fn at(&self, x: u32, y: u32) -> [u8; 3] {
        let i = ((y * self.width + x) * 4) as usize;
        [self.rgba[i], self.rgba[i + 1], self.rgba[i + 2]]
    }
}

/// A fixture element's region: `[left, top, right, bottom]` in device
/// pixels, right and bottom excluded.
pub struct Region {
    /// The element, for messages.
    pub label: String,
    /// Its border box, one pixel larger all round.
    pub rect: [u32; 4],
}

fn distance(a: [u8; 3], b: [u8; 3]) -> u8 {
    (0..3).map(|i| a[i].abs_diff(b[i])).max().unwrap_or(0)
}

fn flat(theirs: &Image<'_>, x: u32, y: u32) -> bool {
    let centre = theirs.at(x, y);
    (x - 1..=x + 1)
        .all(|i| (y - 1..=y + 1).all(|j| distance(theirs.at(i, j), centre) <= FLAT_NEIGHBOURS))
}

/// Every broken rule, each as a line naming where.
#[must_use]
pub fn compare(ours: &Image<'_>, theirs: &Image<'_>, regions: &[Region]) -> Vec<String> {
    let mut failures = pixel_failures(ours, theirs);
    failures.extend(
        regions
            .iter()
            .filter_map(|region| region_failure(ours, theirs, region)),
    );
    failures
}

fn pixel_failures(ours: &Image<'_>, theirs: &Image<'_>) -> Vec<String> {
    let mut failures = Vec::new();
    for y in 1..theirs.height - 1 {
        for x in 1..theirs.width - 1 {
            let d = distance(ours.at(x, y), theirs.at(x, y));
            if (d > FLAT_TOLERANCE && flat(theirs, x, y)) || d > EDGE_BOUND {
                failures.push(format!(
                    "({x}, {y}) ours {:?}, Chromium's {:?}",
                    ours.at(x, y),
                    theirs.at(x, y)
                ));
            }
        }
    }
    failures
}

fn region_failure(ours: &Image<'_>, theirs: &Image<'_>, region: &Region) -> Option<String> {
    let [left, top, right, bottom] = region.rect;
    let (right, bottom) = (right.min(theirs.width), bottom.min(theirs.height));
    let strong = (top..bottom)
        .flat_map(|y| (left..right).map(move |x| (x, y)))
        .filter(|(x, y)| strongly_off(ours, theirs, *x, *y))
        .count();
    (strong > BUDGET).then(|| {
        format!("{}: {strong} pixels are off by more than half the local contrast; at most {BUDGET} may be", region.label)
    })
}

fn strongly_off(ours: &Image<'_>, theirs: &Image<'_>, x: u32, y: u32) -> bool {
    let d = distance(ours.at(x, y), theirs.at(x, y));
    let contrast = contrast(ours, x, y).max(contrast(theirs, x, y));
    d > STRONG_FLOOR && u32::from(d) * 2 > u32::from(contrast)
}

/// The largest channel range in the 5x5 neighbourhood of (x, y).
fn contrast(image: &Image<'_>, x: u32, y: u32) -> u8 {
    let (mut low, mut high) = ([255u8; 3], [0u8; 3]);
    for j in y.saturating_sub(2)..=(y + 2).min(image.height - 1) {
        for i in x.saturating_sub(2)..=(x + 2).min(image.width - 1) {
            let p = image.at(i, j);
            for c in 0..3 {
                low[c] = low[c].min(p[c]);
                high[c] = high[c].max(p[c]);
            }
        }
    }
    (0..3).map(|c| high[c] - low[c]).max().unwrap_or(0)
}

#[cfg(test)]
#[path = "paint_check_tests.rs"]
mod tests;
