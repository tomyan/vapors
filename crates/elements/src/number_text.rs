//! A number field's text (3-E3c): written to its `value` by an edit,
//! or set with the caret at the end by a step or commit, and where
//! focus leaves the caret.

use vapors_dom::{NodeId, PropertyValue};

use crate::document::Document;

impl Document {
    /// Sets the field's text with the caret at its end, as setting
    /// `value` in Chromium puts it; whether it changed.
    pub(crate) fn set_number_text(&mut self, node: NodeId, text: String) -> bool {
        let changed = self.write_number_text(node, text);
        self.editor_from_text(node);
        // the method path is not general over the driver's lifetime
        #[allow(clippy::redundant_closure_for_method_calls)]
        self.move_selection(node, |d| d.move_to_text_end());
        changed
    }

    /// Writes the field's text to its `value`; whether it changed.
    pub(crate) fn write_number_text(&mut self, node: NodeId, text: String) -> bool {
        if self.number_field(node).is_some_and(|f| f.text == text) {
            return false;
        }
        self.note_change();
        if let Some(element) = self.arena.get_mut(node, "number").element_mut() {
            element
                .properties
                .insert("value".to_owned(), PropertyValue::Text(text));
        }
        self.boxes.marks.mark(node);
        // the text decides `:invalid`
        self.mark_flag(node, vapors_css::tree::States::INVALID);
        true
    }

    /// Focus arrived at `node` by `keyboard` or not: a number field
    /// remembers what it would commit against; keyboard focus selects
    /// its text whole, other focus puts the caret at the end.
    pub(crate) fn number_focused(&mut self, node: NodeId, keyboard: bool) {
        let Some(field) = self.number_field(node) else {
            return;
        };
        let committed = self.last_committed(node, &field);
        self.remember_committed(node, committed);
        self.move_selection(node, |d| {
            if keyboard {
                d.select_all();
            } else {
                d.move_to_text_end();
            }
        });
    }
}
