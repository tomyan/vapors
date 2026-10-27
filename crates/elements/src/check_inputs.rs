//! The refusals of form controls (3-E3): an input of a type not built
//! yet, a `disabled` that would mean something other than it says, and
//! a `value` or `checked` property of a shape no input takes.

use crate::node::Element;

/// A `disabled` attribute disables by being present, as in HTML, so
/// `disabled="false"` disables too; a `.wisp` that sets it from an
/// expression would get exactly that. Refused where `disabled` has an
/// effect, a button or an input (elsewhere HTML ignores it), until the
/// tree can remove an attribute (a `Dom` change).
pub(crate) fn disabled_value(tag: &str, value: Option<&String>) -> Option<String> {
    if !matches!(tag, "button" | "input") {
        return None;
    }
    match value.map(String::as_str) {
        Some("false") => Some(format!(
            "<{tag} disabled=\"false\"> is disabled: an HTML boolean attribute disables by being present; leave it out to enable, since the tree cannot remove an attribute yet"
        )),
        _ => None,
    }
}

/// Whether `element`'s `type` attribute is `kind`, compared as HTML
/// compares it (ASCII case-insensitively); an `input` without one is
/// `text`.
pub(crate) fn is_type(element: &Element, kind: &str) -> bool {
    element
        .attributes
        .get("type")
        .map_or("text", String::as_str)
        .eq_ignore_ascii_case(kind)
}

/// The input types built and where the rest arrive (3-E3).
const INPUT_TYPES: &[(&str, Option<&str>)] = &[
    ("range", None),
    ("checkbox", None),
    ("number", None),
    ("text", Some("3-E6, text editing")),
];

/// An `input` of a type not built yet, naming the slice; else a
/// `value` property of a shape no input's value takes.
pub(crate) fn input_refusal(element: &Element) -> Option<String> {
    if element.tag != "input" {
        return None;
    }
    input_type_refusal(element).or_else(|| value_property_refusal(element))
}

fn input_type_refusal(element: &Element) -> Option<String> {
    let slice = INPUT_TYPES
        .iter()
        .find(|(kind, _)| is_type(element, kind))
        .map_or(Some("no slice yet"), |(_, slice)| *slice)?;
    let kind = element
        .attributes
        .get("type")
        .map_or("text", String::as_str);
    let built: Vec<&str> = INPUT_TYPES
        .iter()
        .filter(|(_, s)| s.is_none())
        .map(|(k, _)| *k)
        .collect();
    Some(format!(
        "<input type=\"{kind}\"> is not implemented by vapors-elements yet: it arrives in {slice} (docs/design/03-events-focus-input-ime.md); the types built are {}",
        built.join(", ")
    ))
}

/// A boolean `value` is no value at all, and a number that is NaN or
/// infinite is one a signal computed wrongly (the DOM would empty the
/// value and say nothing); both are refused naming the fix, as is a
/// `checked` that is not a boolean.
fn value_property_refusal(element: &Element) -> Option<String> {
    match (
        element.properties.get("value"),
        element.properties.get("checked"),
    ) {
        (Some(vapors_dom::PropertyValue::Bool(_)), _) => Some(
            "its `value` property is a boolean; an input's value is a number or a text (set_property with PropertyValue::Number or Text)".to_owned(),
        ),
        (Some(vapors_dom::PropertyValue::Number(n)), _) if !n.is_finite() => Some(format!(
            "its `value` property is {n}, which is not a number an input can hold; the signal bound to it computed a non-finite value, so fix the computation"
        )),
        (_, Some(vapors_dom::PropertyValue::Text(_) | vapors_dom::PropertyValue::Number(_))) => Some(
            "its `checked` property is not a boolean; a checkbox's state is PropertyValue::Bool".to_owned(),
        ),
        _ => None,
    }
}
