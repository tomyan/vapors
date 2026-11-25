//! The placeholder of an empty text or number field (`docs/placeholder.md`):
//! which inputs take one, and its text. Its colour is a style
//! (`-vapors-placeholder-color`, from `::placeholder { color }`).

use crate::node::Element;

/// Whether `element` is an input that paints a placeholder: one with an
/// editor (`text_field::is_edited`), a text or number field.
pub(crate) fn takes_placeholder(element: &Element) -> bool {
    crate::text_field::is_edited(element)
}

/// What `element` shows while it is empty: its `placeholder`, as the
/// HTML parser gives it with line breaks removed (HTML, 4.10.5.1.2);
/// `None` when it has none, or is not an input that shows one. A
/// placeholder of only line breaks is empty, so shows nothing, as in
/// Chromium.
pub(crate) fn text_of(element: &Element) -> Option<String> {
    let text = element.attributes.get("placeholder")?;
    let text: String = text.chars().filter(|c| !matches!(c, '\n' | '\r')).collect();
    (takes_placeholder(element) && !text.is_empty()).then_some(text)
}
