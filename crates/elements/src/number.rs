//! A number field's numbers (3-E3c): `min` and `max` (none without
//! one), `step` (1 without one, `any` for none), and its text, which is
//! what the user typed or a signal set. The DOM's `value` is the text
//! when it is a valid floating-point number, else empty; a field whose
//! text is not a number, or is a number outside the range, is
//! `:invalid`. On commit (Enter, blur) a number outside the range is
//! clamped into it and text that is no number reverts to the last
//! committed value, vapors' own rule (Chromium keeps the text): a
//! `change` always carries a number.

use std::collections::BTreeMap;

use vapors_dom::PropertyValue;

use crate::range::{aligned, parse, written};

/// A number field's numbers and text.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Field {
    pub(crate) min: Option<f64>,
    pub(crate) max: Option<f64>,
    /// `None` for `step="any"`.
    pub(crate) step: Option<f64>,
    /// The text shown: the `value` property once set, else the `value`
    /// attribute, else empty.
    pub(crate) text: String,
}

impl Field {
    /// The field an element's attributes and properties describe.
    pub(crate) fn of(
        attributes: &BTreeMap<String, String>,
        properties: &BTreeMap<String, PropertyValue>,
    ) -> Self {
        let number = |name: &str| attributes.get(name).and_then(|text| parse(text));
        Self {
            min: number("min"),
            max: number("max"),
            step: step_of(attributes.get("step").map(String::as_str)),
            text: text_of(attributes, properties),
        }
    }

    /// The text as a number, when it is a valid floating-point number.
    pub(crate) fn number(&self) -> Option<f64> {
        parse(&self.text)
    }

    /// The DOM's `value`: the text when it is a number, else empty.
    pub(crate) fn value(&self) -> String {
        if self.number().is_some() {
            self.text.clone()
        } else {
            String::new()
        }
    }

    /// Whether the field is `:invalid`: text that is no number, or a
    /// number outside the range (a step mismatch is not flagged yet).
    pub(crate) fn invalid(&self) -> bool {
        if self.text.is_empty() {
            return false;
        }
        match self.number() {
            // clamping returns `n` itself when it is in range, so the
            // comparison is exact
            #[allow(clippy::float_cmp)]
            Some(n) => self.clamped(n) != n,
            None => true,
        }
    }

    /// `n` inside the range.
    pub(crate) fn clamped(&self, n: f64) -> f64 {
        let low = self.min.map_or(n, |min| n.max(min));
        self.max.map_or(low, |max| low.min(max))
    }

    /// The number the text commits to: a number clamped into the range;
    /// `None` for text that is no number, which reverts.
    pub(crate) fn committed(&self) -> Option<f64> {
        self.number().map(|n| self.clamped(n))
    }

    /// The number `steps` steps from the current one (from `fallback`
    /// when the text is no number), clamped; one step is the `step`, or
    /// 1 for `step="any"`.
    pub(crate) fn stepped(&self, steps: f64, fallback: f64) -> f64 {
        let from = self.number().unwrap_or(fallback);
        let step = self.step.unwrap_or(1.0);
        self.clamped(aligned(from + steps * step, Some(step), from))
    }
}

/// The text shown: the `value` property once set, else the attribute.
fn text_of(
    attributes: &BTreeMap<String, String>,
    properties: &BTreeMap<String, PropertyValue>,
) -> String {
    match properties.get("value") {
        Some(PropertyValue::Number(n)) if n.is_finite() => written(*n),
        Some(PropertyValue::Text(text)) => text.clone(),
        // a non-finite number and a Bool are refused by `check`
        Some(PropertyValue::Number(_) | PropertyValue::Bool(_)) => String::new(),
        None => attributes.get("value").cloned().unwrap_or_default(),
    }
}

/// `step`: `any` for none, a positive number, else the default of 1.
fn step_of(attribute: Option<&str>) -> Option<f64> {
    match attribute.map(str::trim) {
        Some(any) if any.eq_ignore_ascii_case("any") => None,
        Some(text) => Some(parse(text).filter(|s| *s > 0.0).unwrap_or(1.0)),
        None => Some(1.0),
    }
}

/// Whether `element` is a number field whose text is invalid.
pub(crate) fn invalid_of(element: &crate::node::Element) -> bool {
    element.tag == "input"
        && crate::check::is_type(element, "number")
        && Field::of(&element.attributes, &element.properties).invalid()
}

/// Whether a typed character belongs in a number field: Chromium
/// accepts digits, signs, the decimal point and the exponent's `e`,
/// and drops the rest.
pub(crate) fn typeable(c: char) -> bool {
    c.is_ascii_digit() || matches!(c, '+' | '-' | '.' | 'e' | 'E')
}

#[cfg(test)]
#[path = "number_tests.rs"]
mod tests;
