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
    /// The fragments of the bytes in `range`, top to bottom: on each
    /// line, the part of `range` up to the line's end, without the
    /// collapsible spaces the line breaks after (they take no room, as
    /// in CSS); none for an empty range.
    #[must_use]
    pub fn fragments(&self, range: Range<usize>) -> Vec<Fragment> {
        self.lines()
            .enumerate()
            .filter_map(|(line, info)| {
                let end = trimmed_end(self.text(), info.text_range.clone());
                let part = range.start.max(info.text_range.start)..range.end.min(end);
                self.extent(part, line)
            })
            .collect()
    }

    /// The x extent of `part`, all on line `line`.
    #[allow(clippy::cast_possible_truncation)] // Parley's f64 boxes come from f32 positions
    fn extent(&self, part: Range<usize>, line: usize) -> Option<Fragment> {
        if part.is_empty() {
            return None;
        }
        let inner = self.inner();
        let selection = Selection::new(
            Cursor::from_byte_index(inner, part.start, Affinity::Downstream),
            Cursor::from_byte_index(inner, part.end, Affinity::Upstream),
        );
        let rects = selection.geometry(inner);
        let (left, right) = rects
            .iter()
            .filter(|(_, on)| *on == line)
            .fold((f64::MAX, f64::MIN), |(l, r), (rect, _)| {
                (l.min(rect.x0), r.max(rect.x1))
            });
        (left < right).then_some(Fragment {
            line,
            x: left as f32,
            width: (right - left) as f32,
        })
    }
}

/// The end of `line` in `text` without its trailing spaces.
fn trimmed_end(text: &str, line: Range<usize>) -> usize {
    line.start + text[line].trim_end_matches(' ').len()
}

#[cfg(test)]
#[path = "layout_ranges_tests.rs"]
mod tests;
