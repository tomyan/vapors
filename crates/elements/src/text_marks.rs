//! A focused text field's caret and selection, laid out from its
//! editor for paint (E6a).

use vapors_dom::NodeId;
use vapors_scene::Color;
use vapors_text::parley::{Affinity, Cursor, PlainEditor, Selection};

use crate::document::Document;

/// The caret and selection of a focused field, in CSS px from the
/// text's origin, for paint.
#[derive(Clone, Debug, PartialEq, Default)]
pub(crate) struct TextMarks {
    /// The caret's rect, when the selection is collapsed.
    pub(crate) caret: Option<[f32; 4]>,
    /// The selection's rects.
    pub(crate) selection: Vec<[f32; 4]>,
    /// The rects of the text an IME is composing, underlined.
    pub(crate) composing: Vec<[f32; 4]>,
}

impl Document {
    /// The focused text or number field's caret and selection, laid out now.
    pub(crate) fn focused_text_marks(&mut self) -> Option<(NodeId, TextMarks)> {
        let node = self
            .active_element()
            .filter(|node| self.is_edited(*node) && self.editors.contains_key(node))?;
        self.editor(node);
        let Document { fonts, editors, .. } = self;
        let editor = editors.get_mut(&node)?;
        fonts.editor_driver(&mut editor.plain).refresh_layout();
        let mut marks = marks_of(&editor.plain);
        marks.composing = composing_rects(&editor.plain, editor.composing.clone());
        Some((node, marks))
    }
}

/// An editor's caret (when its selection is collapsed) and selection
/// rects, laid out already.
fn marks_of(plain: &PlainEditor<Color>) -> TextMarks {
    TextMarks {
        caret: plain
            .raw_selection()
            .is_collapsed()
            .then(|| plain.cursor_geometry(1.0).map(rect))
            .flatten(),
        selection: plain
            .selection_geometry()
            .into_iter()
            .map(|(b, _)| rect(b))
            .collect(),
        composing: Vec::new(),
    }
}

/// The rects of `range` (bytes) in the editor's laid-out text.
fn composing_rects(
    plain: &PlainEditor<Color>,
    range: Option<std::ops::Range<usize>>,
) -> Vec<[f32; 4]> {
    let (Some(range), Some(layout)) = (range, plain.try_layout()) else {
        return Vec::new();
    };
    let at = |index, affinity| Cursor::from_byte_index(layout, index, affinity);
    Selection::new(
        at(range.start, Affinity::Downstream),
        at(range.end, Affinity::Upstream),
    )
    .geometry(layout)
    .into_iter()
    .map(|(b, _)| rect(b))
    .collect()
}

/// A Parley box as `[x, y, width, height]`.
// Parley's geometry is f64 CSS px within a field, which f32 holds
#[allow(clippy::cast_possible_truncation)]
fn rect(b: vapors_text::parley::BoundingBox) -> [f32; 4] {
    [
        b.x0 as f32,
        b.y0 as f32,
        (b.x1 - b.x0) as f32,
        (b.y1 - b.y0) as f32,
    ]
}
