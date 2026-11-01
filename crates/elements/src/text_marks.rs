//! A focused text field's caret and selection, laid out from its
//! editor for paint (E6a).

use vapors_dom::NodeId;

use crate::document::Document;

/// The caret and selection of a focused field, in CSS px from the
/// text's origin, for paint.
#[derive(Clone, Debug, PartialEq, Default)]
pub(crate) struct TextMarks {
    /// The caret's rect, when the selection is collapsed.
    pub(crate) caret: Option<[f32; 4]>,
    /// The selection's rects.
    pub(crate) selection: Vec<[f32; 4]>,
}

impl Document {
    /// The focused text field's caret and selection, laid out now.
    pub(crate) fn focused_text_marks(&mut self) -> Option<(NodeId, TextMarks)> {
        let node = self
            .active_element()
            .filter(|node| self.is_text_input(*node) && self.editors.contains_key(node))?;
        self.editor(node);
        let Document { fonts, editors, .. } = self;
        let editor = editors.get_mut(&node)?;
        let mut driver = fonts.editor_driver(&mut editor.plain);
        driver.refresh_layout();
        // Parley's geometry is f64 CSS px within a field, which f32 holds
        #[allow(clippy::cast_possible_truncation)]
        let rect = |b: vapors_text::parley::BoundingBox| {
            [
                b.x0 as f32,
                b.y0 as f32,
                (b.x1 - b.x0) as f32,
                (b.y1 - b.y0) as f32,
            ]
        };
        let marks = TextMarks {
            caret: editor
                .plain
                .raw_selection()
                .is_collapsed()
                .then(|| editor.plain.cursor_geometry(1.0).map(rect))
                .flatten(),
            selection: editor
                .plain
                .selection_geometry()
                .into_iter()
                .map(|(b, _)| rect(b))
                .collect(),
        };
        Some((node, marks))
    }
}
