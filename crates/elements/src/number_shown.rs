//! What a number field shows (touché, ux-review-1). A value set as a
//! number shows at its `step` attribute's precision: "0.50" for 0.5000001
//! by 0.01, "7" for 7.1465 by 1, halves rounded away from zero as
//! JavaScript's `toFixed`, never "-0". Typed text, a value outside the
//! range (so its `:invalid` style has a visible reason), and a field with
//! no usable `step` attribute (none, `any`, not a positive number) or a
//! step finer than six decimals show the full text. The DOM's `value`,
//! `input` and `change` keep the full number; editing starts from what
//! is shown (`number_input.rs`).
//!
//! DEVIATION from Chromium, which paints the number as set: an app
//! holding full-precision numbers (touché's adjustments) would otherwise
//! have to round its model to keep the field readable, and a rounded
//! value written back would change the model. Retire it when vapors has
//! a way for the app to give the field's display format itself.

use std::collections::BTreeMap;

use vapors_dom::PropertyValue;

use crate::number::Field;
use crate::range::parse;

/// The most decimals a step is shown with; a finer step shows in full.
const MOST_DECIMALS: i32 = 6;

/// What `field`, described by `attributes` and `properties`, shows.
pub(crate) fn shown(
    field: &Field,
    attributes: &BTreeMap<String, String>,
    properties: &BTreeMap<String, PropertyValue>,
) -> String {
    let Some(PropertyValue::Number(value)) = properties.get("value") else {
        return field.text.clone();
    };
    match declared_decimals(attributes) {
        Some(decimals) if value.is_finite() && !field.invalid() => at_decimals(*value, decimals),
        _ => field.text.clone(),
    }
}

/// The decimals the `step` attribute has, when it is a positive number
/// with at most [`MOST_DECIMALS`].
fn declared_decimals(attributes: &BTreeMap<String, String>) -> Option<i32> {
    let step = attributes
        .get("step")
        .and_then(|text| parse(text.trim()))
        .filter(|step| *step > 0.0)?;
    (0..=MOST_DECIMALS).find(|&d| {
        let scaled = step * 10f64.powi(d);
        (scaled - scaled.round()).abs() < 1e-9
    })
}

/// `value` with `decimals` decimals, halves away from zero, never "-0".
fn at_decimals(value: f64, decimals: i32) -> String {
    let scale = 10f64.powi(decimals);
    let rounded = (value * scale).round() / scale;
    // a value near f64's limit overflows when scaled; it has no decimals
    let rounded = if rounded.is_finite() { rounded } else { value };
    let places = usize::try_from(decimals).unwrap_or(0);
    let text = format!("{rounded:.places$}");
    match text.strip_prefix('-') {
        Some(rest) if rest.chars().all(|c| c == '0' || c == '.') => rest.to_owned(),
        _ => text,
    }
}
