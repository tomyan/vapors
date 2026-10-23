//! Where a byte range of a laid-out paragraph sits: one fragment per
//! line it is on, for an inline element's boxes, background and hit
//! regions.

use std::ops::Range;

use parley::{Affinity, Cursor, Selection};

use crate::layout::Layout;

/// A range's extent on one line, in device pixels from the layout's
/// origin.
#[derive(Clone, Debug, PartialEq)]
pub struct Fragment {
    /// The line's index, top to bottom.
    pub line: usize,
    /// Left edge.
    pub x: f32,
    /// Width.
    pub width: f32,
}

impl Layout {
    /// The fragments of the bytes in `range`, top to bottom; none for
    /// an empty range.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)] // Parley's f64 boxes come from f32 positions
    pub fn fragments(&self, range: Range<usize>) -> Vec<Fragment> {
        if range.is_empty() {
            return Vec::new();
        }
        let inner = self.inner();
        let selection = Selection::new(
            Cursor::from_byte_index(inner, range.start, Affinity::Downstream),
            Cursor::from_byte_index(inner, range.end, Affinity::Upstream),
        );
        selection
            .geometry(inner)
            .into_iter()
            .map(|(rect, line)| Fragment {
                line,
                x: rect.x0 as f32,
                width: (rect.x1 - rect.x0) as f32,
            })
            .collect()
    }
}

#[cfg(test)]
#[path = "layout_ranges_tests.rs"]
mod tests;
