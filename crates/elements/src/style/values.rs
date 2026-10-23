//! Reading computed values: each property is read through [`Reader`],
//! which records a refusal (property, value, reason) for a value the
//! tree does not implement and carries on with a stand-in, so one
//! update reports every refusal at once. A refusal always fails the
//! update; the stand-in is never drawn.

use vapors_css::cascade::ComputedStyle;
use vapors_css::value::{LengthUnit, Value};

/// A value the tree does not implement.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Refusal {
    pub(crate) property: &'static str,
    pub(crate) value: String,
    pub(crate) reason: String,
}

/// Reads one element's computed style, collecting refusals.
pub(crate) struct Reader<'a> {
    style: &'a ComputedStyle,
    pub(crate) refusals: Vec<Refusal>,
}

impl<'a> Reader<'a> {
    pub(crate) fn new(style: &'a ComputedStyle) -> Self {
        Self {
            style,
            refusals: Vec::new(),
        }
    }

    /// The value of `property` through `read`; on a refusal, records it
    /// and returns `stand_in`.
    pub(crate) fn read<T>(
        &mut self,
        property: &'static str,
        read: impl FnOnce(&Value) -> Result<T, String>,
        stand_in: T,
    ) -> T {
        let value = self.value(property);
        match read(value) {
            Ok(result) => result,
            Err(reason) => {
                self.refusals.push(Refusal {
                    property,
                    value: value.to_css(),
                    reason,
                });
                stand_in
            }
        }
    }

    /// The computed value of `property`.
    ///
    /// # Panics
    /// When the CSS crate has no such longhand: a typo in this crate.
    pub(crate) fn value(&self, property: &'static str) -> &'a Value {
        self.style
            .get_by_name(property)
            .unwrap_or_else(|| panic!("`{property}` is not a longhand in vapors-css's table"))
    }
}

/// A length or a percentage of something layout knows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Amount {
    /// CSS pixels.
    Px(f32),
    /// A fraction, 0.5 for 50%.
    Fraction(f32),
}

/// A computed length or percentage. `calc()` mixing % with lengths is
/// refused until layout can resolve it (F2c S4).
#[allow(clippy::cast_possible_truncation)] // CSS lengths are far inside f32
pub(crate) fn amount(value: &Value) -> Result<Amount, String> {
    match value {
        Value::Length(length) if length.unit == LengthUnit::Px => {
            Ok(Amount::Px(length.value as f32))
        }
        Value::Length(length) if length.unit == LengthUnit::Percent => {
            Ok(Amount::Fraction((length.value / 100.0) as f32))
        }
        Value::Calc(_) => {
            Err("is a calc() with a % term, which arrives in F2c S4 (docs/F2c.md)".to_owned())
        }
        other => Err(format!(
            "is not a length or percentage the tree reads ({other:?})"
        )),
    }
}

/// The keyword, or a refusal naming the value.
pub(crate) fn keyword(value: &Value) -> Result<&str, String> {
    value
        .as_keyword()
        .ok_or_else(|| format!("is not a keyword ({value:?})"))
}

/// `name` arrives in `slice`: the reason for a refused value.
pub(crate) fn later(slice: &str) -> String {
    format!("arrives in {slice} (docs/F2c.md)")
}

/// A keyword the property's grammar allows but this crate does not
/// know: the grammar gained a value and the mapping was not updated.
pub(crate) fn unknown(keyword: &str) -> String {
    format!(
        "`{keyword}` is not a value vapors-elements knows; the CSS grammar and the mapping in vapors-elements/src/style disagree"
    )
}
