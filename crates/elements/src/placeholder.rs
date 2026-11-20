//! The placeholder of an empty text or number field (`docs/placeholder.md`):
//! which inputs take one, its text, and its colour.

use vapors_scene::Color;

use crate::node::Element;

/// The placeholder's colour: Chromium's, `#757575` (`rgb(117, 117, 117)`,
/// what `getComputedStyle(input, '::placeholder').color` reads in its
/// light scheme; its html.css has `-internal-light-dark(#757575,
/// #A9A9A9)`). `::placeholder { color }` is left (`docs/placeholder.md`).
pub(crate) const COLOR: Color = Color {
    r: 117.0 / 255.0,
    g: 117.0 / 255.0,
    b: 117.0 / 255.0,
    a: 1.0,
};

/// Whether `element` is an input that paints a placeholder: a text
/// field, or an `input type=number`.
pub(crate) fn takes_placeholder(element: &Element) -> bool {
    element.tag == "input"
        && (crate::text_field::is_text_input(element) || crate::check::is_type(element, "number"))
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
