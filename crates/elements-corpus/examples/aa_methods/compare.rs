//! One cell of ours against Chromium's under the `paint_check` rules
//! (`vapors_elements_corpus::paint_check`): whether it passes, how many
//! pixels break rules 1 and 2, how many are strongly off (rule 3, when
//! over its budget), and the largest channel difference.

use vapors_elements_corpus::paint_check::{BUDGET, Image, Region, compare};

/// What the rules found in one cell.
pub struct Found {
    pub pass: bool,
    pub pixels: usize,
    /// Strongly-off pixels, when over the budget.
    pub strong: Option<usize>,
    pub max: u8,
}

/// Cell `index` (`side` device pixels square) of `ours` against
/// `theirs`, both `width` device pixels wide.
pub fn cell(ours: &[u8], theirs: &[u8], width: u32, side: u32, index: u32) -> Found {
    let (a, b) = (
        crop(ours, width, side, index),
        crop(theirs, width, side, index),
    );
    let image = |rgba| Image {
        width: side,
        height: side,
        rgba,
    };
    let region = Region {
        label: format!("cell {index}"),
        rect: [0, 0, side, side],
    };
    let failures = compare(&image(&a), &image(&b), &[region]);
    let strong = failures.iter().find_map(|f| strong_count(f));
    Found {
        pass: failures.is_empty(),
        pixels: failures.len() - usize::from(strong.is_some()),
        strong,
        max: a
            .iter()
            .zip(&b)
            .map(|(x, y)| x.abs_diff(*y))
            .max()
            .unwrap_or(0),
    }
}

/// The strongly-off count a region failure names (rule 3: over
/// `BUDGET`).
fn strong_count(failure: &str) -> Option<usize> {
    let (_, rest) = failure.split_once(": ")?;
    let count = rest.split_whitespace().next()?.parse().ok()?;
    (count > BUDGET).then_some(count)
}

fn crop(rgba: &[u8], width: u32, side: u32, index: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity((side * side * 4) as usize);
    for y in 0..side {
        let start = ((y * width + index * side) * 4) as usize;
        out.extend_from_slice(&rgba[start..start + (side * 4) as usize]);
    }
    out
}
