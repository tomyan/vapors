//! The word a double click selects (E6a), as Chromium selects it: the
//! word after the caret position nearest the point, so a point just
//! past a word's last letter selects the space after it; past the end
//! of the text, the last word. Parley's own `select_word_at_point`
//! takes the cluster under the point instead, which picks the word
//! before.

use std::ops::Range;

use vapors_scene::Color;
use vapors_text::parley::{Cluster, Cursor, Layout};

/// The byte range of the word a double click at `x`, `y` selects.
pub(crate) fn word_at(layout: &Layout<Color>, x: f32, y: f32) -> Range<usize> {
    let index = Cursor::from_point(layout, x, y).index();
    let Some(cluster) = Cluster::from_byte_index(layout, index)
        .or_else(|| Cluster::from_byte_index(layout, index.checked_sub(1)?))
    else {
        return index..index;
    };
    let start = if cluster.is_word_boundary() {
        cluster
    } else {
        cluster.previous_logical_word().unwrap_or(cluster)
    };
    let end = start
        .next_logical_word()
        .map_or(layout.text_len(), |next| next.text_range().start);
    start.text_range().start..end
}
