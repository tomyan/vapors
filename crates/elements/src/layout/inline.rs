//! An inline run: the text of a block's consecutive inline content
//! (text nodes and `display: inline` elements, nested), collapsed as
//! `white-space: normal` collapses it across element boundaries, with
//! each inline element's byte range and style. Taffy lays a run out as
//! one anonymous block (CSS 2 section 9.2.1.1).

use std::ops::Range;

use vapors_dom::NodeId;

use crate::style::TextStyle;

/// One inline element's part of a run.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct InlineSpan {
    pub(crate) node: NodeId,
    /// Bytes of the run's text inside the element, its descendants'
    /// included.
    pub(crate) range: Range<usize>,
    pub(crate) style: TextStyle,
    /// `visibility: visible`.
    pub(crate) visible: bool,
}

/// A run's text, the block's text style and visibility, and its inline
/// elements outermost first, so an inner one's style applies over an
/// outer one's.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct InlineRun {
    pub(crate) text: String,
    pub(crate) style: TextStyle,
    pub(crate) visible: bool,
    pub(crate) spans: Vec<InlineSpan>,
}

/// Builds a run piece by piece.
pub(crate) struct Collector {
    run: InlineRun,
}

impl Collector {
    pub(crate) fn new(style: TextStyle, visible: bool) -> Self {
        Self {
            run: InlineRun {
                text: String::new(),
                style,
                visible,
                spans: Vec::new(),
            },
        }
    }

    /// Appends a text node's text: each run of whitespace becomes one
    /// space, and none starts the run or follows another space.
    pub(crate) fn push_text(&mut self, text: &str) {
        for c in text.chars() {
            if !is_collapsible(c) {
                self.run.text.push(c);
            } else if !self.run.text.is_empty() && !self.run.text.ends_with(' ') {
                self.run.text.push(' ');
            }
        }
    }

    /// Opens an inline element; returns where its bytes start, for
    /// [`Collector::close`].
    pub(crate) fn open(&mut self) -> usize {
        self.run.spans.len()
    }

    /// Records the element opened at `slot`, holding the bytes from
    /// `start` to here.
    pub(crate) fn close(&mut self, slot: usize, span: InlineSpan) {
        self.run.spans.insert(slot, span);
    }

    /// Where the next byte will go.
    pub(crate) fn position(&self) -> usize {
        self.run.text.len()
    }

    /// The run, its trailing space removed and its ranges kept inside
    /// the text; `None` when it holds no text.
    pub(crate) fn finish(mut self) -> Option<InlineRun> {
        if self.run.text.ends_with(' ') {
            self.run.text.pop();
        }
        if self.run.text.is_empty() {
            return None;
        }
        let end = self.run.text.len();
        for span in &mut self.run.spans {
            span.range = span.range.start.min(end)..span.range.end.min(end);
        }
        Some(self.run)
    }
}

/// White space `white-space: normal` collapses; a no-break space is
/// not one of these.
fn is_collapsible(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r' | '\u{c}')
}

#[cfg(test)]
#[path = "inline_tests.rs"]
mod tests;
