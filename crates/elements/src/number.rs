//! A number field's numbers (3-E3c): `min` and `max` (none without
//! one), `step` (1 without one, `any` for none), and its text, which is
//! what the user typed or a signal set. The DOM's `value` is the text
//! when it is a valid floating-point number, else empty; a field whose
//! text is not a number, or is a number outside the range, is
//! `:invalid`, with `rangeUnderflow` or `rangeOverflow`. On commit
//! (Enter, blur) a number outside the range is kept as typed, as
//! Chromium keeps it; text that is no number reverts to the last
//! committed value, vapors' own rule (Chromium keeps the text): a
//! `change` always carries a number. A step comes back inside the
//! range, and does nothing when it would move further out (HTML's
//! `stepUp` and `stepDown`).

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

    /// The number the text commits to, in the range or not; `None` for
    /// text that is no number, which reverts.
    pub(crate) fn committed(&self) -> Option<f64> {
        self.number()
    }

    /// Whether the text is a number below `min`.
    pub(crate) fn range_underflow(&self) -> bool {
        matches!((self.number(), self.min), (Some(n), Some(min)) if n < min)
    }

    /// Whether the text is a number above `max`.
    pub(crate) fn range_overflow(&self) -> bool {
        matches!((self.number(), self.max), (Some(n), Some(max)) if n > max)
    }

    /// The number `steps` steps from the current one (from `fallback`
    /// when the text is no number); see [`Field::step_from`].
    pub(crate) fn stepped(&self, steps: f64, fallback: f64) -> f64 {
        self.step_from(self.number().unwrap_or(fallback), steps)
    }

    /// The number `steps` steps from `from`, one step being the `step`,
    /// or 1 for `step="any"`, as HTML's `stepUp` and `stepDown` and
    /// Chromium's keyboard step: inside the range, the result stays on
    /// the steps and at most the last one under `max`, and never moves
    /// against its direction; outside it, a step further out leaves
    /// `from`, and a step back goes to the bound.
    pub(crate) fn step_from(&self, from: f64, steps: f64) -> f64 {
        if let Some(outside) = self.step_outside(from, steps) {
            return outside;
        }
        let step = self.step.unwrap_or(1.0);
        let to = aligned(from + steps * step, Some(step), from);
        let to = self.clamped(self.on_steps_under_max(to));
        // the last step under the max can lie below `from`: a step never
        // moves against its direction, as HTML's stepUp and Chromium
        if (to - from) * steps < 0.0 { from } else { to }
    }

    /// From outside the range, or with no step to take: where it goes.
    fn step_outside(&self, from: f64, steps: f64) -> Option<f64> {
        match (self.min, self.max) {
            _ if steps == 0.0 => Some(from),
            (_, Some(max)) if from > max => Some(if steps > 0.0 { from } else { max }),
            (Some(min), _) if from < min => Some(if steps < 0.0 { from } else { min }),
            _ => None,
        }
    }

    /// `to`, or when it is over `max`, the last step at or under `max`,
    /// counting steps from `min` (else 0).
    fn on_steps_under_max(&self, to: f64) -> f64 {
        let (Some(max), Some(step)) = (self.max, self.step) else {
            return to;
        };
        if to <= max {
            return to;
        }
        let base = self.min.unwrap_or(0.0);
        let steps = ((max - base) / step + 1e-9).floor();
        aligned(base + steps * step, Some(step), base)
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
