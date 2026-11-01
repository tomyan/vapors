//! A text field's editor (E6a): `input type=text`, its text and
//! selection held by Parley's `PlainEditor` (movement and selection by
//! cluster and word, insertion and deletion), one per field, made when
//! the field is first pressed or focused and kept while the field is
//! in the document. The text is also the element's `value` property,
//! written on every edit and read back when a signal sets it, so the
//! property stays the one answer to "what does the field say"; the
//! editor holds the selection, which the DOM counts in UTF-16 units.

use std::collections::HashMap;

use vapors_dom::{NodeId, PropertyValue};
use vapors_scene::Color;
use vapors_text::parley::{PlainEditor, Selection};

use crate::document::Document;
use crate::node::Element;
use crate::style::TextStyle;

/// One field's editing state.
pub(crate) struct Editor {
    pub(crate) plain: PlainEditor<Color>,
    /// The text at the last `change`, or at focus.
    pub(crate) committed: String,
    /// The selection the last `selectionchange` reported, so the next
    /// fires only when it moved.
    pub(crate) reported: Option<(usize, usize)>,
    /// Whether a `selectionchange` is owed at the next settle.
    pub(crate) pending: bool,
    /// The field's text style the editor was last given, so its
    /// geometry follows a restyle.
    style: Option<TextStyle>,
    /// The field's own undo and redo (E6b).
    pub(crate) history: crate::text_history::History,
    /// The bytes an IME is composing, underlined (E6, the web's input).
    pub(crate) composing: Option<std::ops::Range<usize>>,
}

/// The editors, by field.
pub(crate) type Editors = HashMap<NodeId, Editor>;

/// Whether `element` is a text field: an `input` of type `text`, or
/// without a type.
pub(crate) fn is_text_input(element: &Element) -> bool {
    element.tag == "input" && crate::check::is_type(element, "text")
}

/// The field's text: its `value` property once set, else the attribute.
pub(crate) fn text_of(element: &Element) -> String {
    match element.properties.get("value") {
        Some(PropertyValue::Text(text)) => text.clone(),
        Some(PropertyValue::Number(n)) => crate::range::written(*n),
        // a Bool is refused by `check`
        Some(PropertyValue::Bool(_)) => String::new(),
        None => element.attributes.get("value").cloned().unwrap_or_default(),
    }
}

impl Document {
    /// Whether `node` is a text field.
    pub(crate) fn is_text_input(&self, node: NodeId) -> bool {
        self.arena
            .get(node, "text field")
            .element()
            .is_some_and(is_text_input)
    }

    /// A text field's selection as the DOM counts it (`selectionStart`,
    /// `selectionEnd` in UTF-16 units; equal for a caret); `None` for
    /// an element that is not a text field. A field never pressed or
    /// focused has its caret at 0, as Chromium's does.
    #[must_use]
    pub fn selection(&self, node: NodeId) -> Option<(usize, usize)> {
        if !self.is_text_input(node) {
            return None;
        }
        Some(self.editors.get(&node).map_or((0, 0), |editor| {
            utf16_range(editor.plain.raw_text(), editor.plain.raw_selection())
        }))
    }

    /// The field's editor, made from its text on first use, in the
    /// field's text style as it is now.
    pub(crate) fn editor(&mut self, node: NodeId) -> &mut Editor {
        let style = self.text_style(node);
        if !self.editors.contains_key(&node) {
            let element = self.arena.get(node, "text field").element();
            let text = element.map(text_of).unwrap_or_default();
            let mut editor = PlainEditor::new(16.0);
            editor.set_text(&text);
            self.editors.insert(
                node,
                Editor {
                    plain: editor,
                    committed: text,
                    reported: None,
                    pending: false,
                    style: None,
                    history: crate::text_history::History::default(),
                    composing: None,
                },
            );
        }
        let editor = self.editors.get_mut(&node).expect("inserted above");
        editor.restyle(style);
        editor
    }

    /// The field's used text style, once styled.
    fn text_style(&self, node: NodeId) -> Option<TextStyle> {
        let kept = self.kept.as_ref()?;
        kept.used.get(&node).map(|used| used.text)
    }

    /// Writes the editor's text to the `value` property, so the property
    /// and the editor say the same; whether it changed.
    pub(crate) fn text_from_editor(&mut self, node: NodeId) -> bool {
        let Some(editor) = self.editors.get(&node) else {
            return false;
        };
        let text = editor.plain.text().to_string();
        let same = self
            .arena
            .get(node, "text field")
            .element()
            .is_some_and(|element| text_of(element) == text);
        if same {
            return false;
        }
        if let Some(element) = self.arena.get_mut(node, "text field").element_mut() {
            element
                .properties
                .insert("value".to_owned(), PropertyValue::Text(text));
        }
        self.changed = true;
        true
    }

    /// A signal set the `value` property: the editor, if any, takes the
    /// text (its selection clamped to it) and forgets its undo.
    pub(crate) fn editor_from_text(&mut self, node: NodeId) {
        let Some(text) = self.arena.get(node, "text field").element().map(text_of) else {
            return;
        };
        if let Some(editor) = self
            .editors
            .get_mut(&node)
            .filter(|editor| editor.plain.raw_text() != text)
        {
            editor.plain.set_text(&text);
            // the steps were of a text the field no longer holds
            editor.history = crate::text_history::History::default();
            editor.pending = true;
        }
    }
}

impl Editor {
    /// Gives the editor `style` when it differs from the last it had,
    /// so its geometry is the painted text's.
    fn restyle(&mut self, style: Option<TextStyle>) {
        let Some(style) = style.filter(|style| self.style != Some(*style)) else {
            return;
        };
        let styles = self.plain.edit_styles();
        styles.clear();
        for property in style.for_layout().properties(1.0) {
            styles.insert(property);
        }
        self.style = Some(style);
    }
}

/// `selection`'s range over `text` in UTF-16 code units, start then end.
fn utf16_range(text: &str, selection: &Selection) -> (usize, usize) {
    let units = |byte: usize| text[..byte.min(text.len())].encode_utf16().count();
    let range = selection.text_range();
    (units(range.start), units(range.end))
}
