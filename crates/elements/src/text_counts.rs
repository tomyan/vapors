//! What the last update's text cache did, counted.

use crate::document::Document;

impl Document {
    /// How many Parley layouts the last update built rather than kept
    /// from the one before: what the incremental tests count, and what
    /// the `stress` demo shows. A layout is a shaped run broken into
    /// lines at one width.
    #[must_use]
    pub fn text_layouts_built(&self) -> usize {
        self.texts.built()
    }

    /// How many runs the last update shaped rather than kept: a run is
    /// shaped once per text, styles, visibility and scale, whichever
    /// node holds it, and each layout of it is broken from that one.
    #[must_use]
    pub fn texts_shaped(&self) -> usize {
        self.texts.shaped()
    }
}
