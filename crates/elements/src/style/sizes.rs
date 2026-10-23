//! Sizes: `width`, `height`, their minimums and maximums, and
//! `flex-basis`, as Taffy 0.15 takes them.

use taffy::{Dimension, LengthPercentageAuto};
use vapors_css::value::Value;

use super::box_style::auto_or_amount;
use super::values::{Amount, amount, unknown};

/// Taffy 0.15 takes sizing keywords in `width`, `height` and
/// `flex-basis` only; `min-*` and `max-*` are lengths or `auto`.
const SIZING_LIMIT: &str = "is a sizing keyword in a min or max size, which Taffy 0.15 does not lay out; it takes them in width, height and flex-basis";

pub(super) fn min_size(value: &Value) -> Result<LengthPercentageAuto, String> {
    match value.as_keyword() {
        Some("auto") | None => auto_or_amount(value),
        Some(_) => Err(SIZING_LIMIT.to_owned()),
    }
}

pub(super) fn none_or_amount(value: &Value) -> Result<LengthPercentageAuto, String> {
    match value.as_keyword() {
        Some("none") => Ok(LengthPercentageAuto::auto()),
        Some(_) => Err(SIZING_LIMIT.to_owned()),
        None => auto_or_amount(value),
    }
}

pub(super) fn dimension(value: &Value) -> Result<Dimension, String> {
    match value.as_keyword() {
        Some("auto") => Ok(Dimension::auto()),
        Some("min-content") => Ok(Dimension::min_content()),
        Some("max-content") => Ok(Dimension::max_content()),
        Some("fit-content") => Ok(Dimension::fit_content()),
        Some(other) => Err(unknown(other)),
        None => Ok(match amount(value)? {
            Amount::Px(px) => Dimension::length(px),
            Amount::Fraction(f) => Dimension::percent(f),
            Amount::Calc(calc) => Dimension::calc(calc),
        }),
    }
}

pub(super) fn flex_basis(value: &Value) -> Result<Dimension, String> {
    match value.as_keyword() {
        Some("content") => Ok(Dimension::content()),
        _ => dimension(value),
    }
}
