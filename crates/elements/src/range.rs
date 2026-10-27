//! A range input's numbers (3-E3a), as HTML's value sanitization
//! algorithm for `type=range` gives them: `min` (0 without one), `max`
//! (100 without one, and never below `min`), `step` (1 without one,
//! `any` for none), and the value, clamped to the range and snapped to
//! the step from its base (`min`, else the `value` attribute, else 0),
//! as Chromium's `StepRange` does. The live value is the element's
//! `value` property once set (by a listener's signal or by the user),
//! else its `value` attribute; without either, the middle of the range.

use std::collections::BTreeMap;

use vapors_dom::{NodeId, PropertyValue};

use crate::arena::Arena;

/// The size a range is laid out at without one of its own, CSS px:
/// Chromium's (`fixtures/range.wisp`'s unsized range).
pub(crate) const NATURAL_SIZE: [f32; 2] = [129.0, 16.0];

/// The range `node` is, when it is an `input type=range`.
pub(crate) fn of_element(arena: &Arena, node: NodeId) -> Option<Range> {
    let element = arena.get(node, "range").element()?;
    if element.tag != "input" || !crate::check::is_type(element, "range") {
        return None;
    }
    Some(Range::of(&element.attributes, &element.properties))
}

/// Chromium's default `step` for a range.
const DEFAULT_STEP: f64 = 1.0;

/// A range's numbers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Range {
    pub(crate) min: f64,
    pub(crate) max: f64,
    /// `None` for `step="any"`.
    pub(crate) step: Option<f64>,
    /// Where stepping counts from.
    pub(crate) base: f64,
    /// The value, sanitized.
    pub(crate) value: f64,
}

impl Range {
    /// The range an element's attributes and properties describe.
    pub(crate) fn of(
        attributes: &BTreeMap<String, String>,
        properties: &BTreeMap<String, PropertyValue>,
    ) -> Self {
        let mut range = Self::bounds(attributes);
        // a value property set, even to a text that is no number, is
        // the live value: the attribute is only the initial one
        let given = match properties.get("value") {
            Some(property) => property_number(property),
            None => attributes.get("value").and_then(|text| parse(text)),
        };
        range.value = range.snapped(given.unwrap_or_else(|| range.middle()));
        range
    }

    /// The range's `min`, `max`, `step` and base, before its value.
    fn bounds(attributes: &BTreeMap<String, String>) -> Self {
        let number = |name: &str| attributes.get(name).and_then(|text| parse(text));
        let min = number("min").unwrap_or(0.0);
        Self {
            min,
            max: number("max").unwrap_or(100.0).max(min),
            step: step_of(attributes.get("step").map(String::as_str)),
            base: min_or_value_base(attributes),
            value: 0.0,
        }
    }

    /// `candidate` clamped into the range and snapped to the nearest
    /// step, as HTML's sanitization and Chromium's `ClampValue` do:
    /// a snapped value past the maximum falls to the step below it.
    pub(crate) fn snapped(&self, candidate: f64) -> f64 {
        let clamped = candidate.clamp(self.min, self.max);
        let Some(step) = self.step else {
            return clamped;
        };
        let rounded = self.base + ((clamped - self.base) / step).round() * step;
        if rounded > self.max {
            self.base + ((clamped - self.base) / step).floor() * step
        } else if rounded < self.min {
            self.base + ((clamped - self.base) / step).ceil() * step
        } else {
            rounded
        }
        .clamp(self.min, self.max)
    }

    /// The default value: the middle of the range.
    pub(crate) fn middle(&self) -> f64 {
        self.min + (self.max - self.min) / 2.0
    }

    /// Where the value lies along the range, 0 at `min` and 1 at `max`
    /// (0 for an empty range).
    pub(crate) fn fraction(&self) -> f64 {
        if self.max > self.min {
            (self.value - self.min) / (self.max - self.min)
        } else {
            0.0
        }
    }

    /// The value at `fraction` of the range, snapped.
    pub(crate) fn at_fraction(&self, fraction: f64) -> f64 {
        self.snapped(self.min + fraction.clamp(0.0, 1.0) * (self.max - self.min))
    }

    /// The value `steps` steps from the current one: one keyboard step
    /// is the `step`, or a hundredth of the range for `step="any"`, as
    /// Chromium steps a range.
    pub(crate) fn stepped(&self, steps: f64) -> f64 {
        let step = self.step.unwrap_or((self.max - self.min) / 100.0);
        self.snapped(self.value + steps * step)
    }

    /// The value a Page key moves to: a tenth of the range, or one step
    /// when that is larger, as Chromium's `big_step`.
    pub(crate) fn paged(&self, up: bool) -> f64 {
        let step = self.step.unwrap_or((self.max - self.min) / 100.0);
        let big = ((self.max - self.min) / 10.0).max(step);
        let sign = if up { 1.0 } else { -1.0 };
        self.snapped(self.value + sign * big)
    }
}

/// The value as the DOM's `value` string: the shortest decimal that
/// reads back as the number, as Rust prints it. JavaScript writes
/// numbers of 1e21 and over, and under 1e-6, with an exponent where
/// Rust writes the digits; no range's oracle reaches either.
pub(crate) fn written(value: f64) -> String {
    format!("{value}")
}

/// A valid floating-point number as HTML parses one (no spaces, no
/// leading `+`, no `inf` or `nan`), else `None`.
fn parse(text: &str) -> Option<f64> {
    let digits = text.strip_prefix('-').unwrap_or(text);
    let shaped = !digits.is_empty()
        && digits.starts_with(|c: char| c.is_ascii_digit() || c == '.')
        && digits
            .chars()
            .all(|c| c.is_ascii_digit() || "+-.eE".contains(c));
    if !shaped {
        return None;
    }
    text.parse::<f64>().ok().filter(|n| n.is_finite())
}

/// `step`: `any` for none, a positive number, else the default.
fn step_of(attribute: Option<&str>) -> Option<f64> {
    match attribute.map(str::trim) {
        Some(any) if any.eq_ignore_ascii_case("any") => None,
        Some(text) => Some(parse(text).filter(|s| *s > 0.0).unwrap_or(DEFAULT_STEP)),
        None => Some(DEFAULT_STEP),
    }
}

/// The step base: `min` when it parses, else the `value` attribute
/// when it does, else 0 (HTML's step base algorithm).
fn min_or_value_base(attributes: &BTreeMap<String, String>) -> f64 {
    ["min", "value"]
        .into_iter()
        .find_map(|name| attributes.get(name).and_then(|text| parse(text)))
        .unwrap_or(0.0)
}

/// A `value` property as a number: a number as it is, a text parsed
/// as HTML parses a value, `None` for a text that is not a number.
fn property_number(value: &PropertyValue) -> Option<f64> {
    match value {
        PropertyValue::Number(n) if n.is_finite() => Some(*n),
        PropertyValue::Text(text) => parse(text),
        PropertyValue::Number(_) | PropertyValue::Bool(_) => None,
    }
}

#[cfg(test)]
#[path = "range_tests.rs"]
mod tests;
