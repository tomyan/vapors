//! The frames of the extents index (`frame_extents.rs`) that hold a
//! step spanning every view (a surface, or bounds that are not
//! numbers), counted as such entries come and go: the kept batches
//! search them whatever their container's clip (`kept_batches.rs`).

use super::{Entry, Extents, missing};

impl Extents {
    /// Notes `entry`, put in frame `frame`, when it spans every view.
    pub(super) fn span_in(&mut self, frame: usize, entry: &Entry) {
        if entry.spans() {
            *self.spanning.entry(frame).or_default() += 1;
        }
    }

    /// Notes `entry`, taken out of frame `frame`, when it spans every view.
    pub(super) fn span_out(&mut self, frame: usize, entry: &Entry) {
        if !entry.spans() {
            return;
        }
        let count = self
            .spanning
            .get_mut(&frame)
            .unwrap_or_else(|| missing(entry.place));
        *count -= 1;
        if *count == 0 {
            self.spanning.remove(&frame);
        }
    }
}

impl Entry {
    /// True when the step spans every view: `entry` gives such a step
    /// no top (a surface, or bounds that are not numbers).
    fn spans(&self) -> bool {
        self.top == f32::NEG_INFINITY
    }
}
