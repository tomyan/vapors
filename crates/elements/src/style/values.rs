//! Reading computed values: each property is read through [`Reader`],
//! which records a refusal (property, value, reason, and where it was
//! declared) for a value the tree does not implement and carries on
//! with a stand-in, so one update reports every refusal at once. A
//! refusal always fails the update; the stand-in is never drawn. An
//! inherited value is refused once, at the element that declared it,
//! not again at each descendant that inherits it.

use vapors_css::Position;
use vapors_css::cascade::ComputedStyle;
use vapors_css::property::PropertyId;
use vapors_css::value::{LengthUnit, Value};

/// A value the tree does not implement.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Refusal {
    pub(crate) property: &'static str,
    pub(crate) value: String,
    pub(crate) reason: String,
    /// The winning declaration's sheet (by cascade order) and position;
    /// `None` when no declaration applied here.
    pub(crate) declared: Option<(usize, Position)>,
}

/// An element's computed style and its parent's: whether a refused
/// value is this element's to report.
#[derive(Clone, Copy)]
pub(crate) struct Styles<'a> {
    pub(crate) own: &'a ComputedStyle,
    pub(crate) parent: Option<&'a ComputedStyle>,
}

impl Styles<'_> {
    /// The refusal for `property` at this element, or `None` when the
    /// value is the parent's, inherited without a declaration here: the
    /// ancestor that declared it reports it.
    pub(crate) fn refusal(&self, property: &'static str, reason: String) -> Option<Refusal> {
        let id = PropertyId::from_name(property)
            .unwrap_or_else(|| panic!("`{property}` is not a longhand in vapors-css's table"));
        let declared = self.own.declared_in(id).zip(self.own.declared_at(id));
        let inherited =
            declared.is_none() && self.parent.is_some_and(|p| p.get(id) == self.own.get(id));
        (!inherited).then(|| Refusal {
            property,
            value: self.own.get(id).to_css(),
            reason,
            declared,
        })
    }
}

/// Reads one element's computed style, collecting refusals.
pub(crate) struct Reader<'a> {
    style: &'a ComputedStyle,
    styles: Styles<'a>,
    pub(crate) refusals: Vec<Refusal>,
}

impl<'a> Reader<'a> {
    pub(crate) fn new(styles: Styles<'a>) -> Self {
        Self {
            style: styles.own,
            styles,
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
                self.refusals.extend(self.styles.refusal(property, reason));
                stand_in
            }
        }
    }

    /// Records a refusal of `property`'s value for `reason` (unless the
    /// value is inherited unchanged, which its declarer reports).
    pub(crate) fn refuse(&mut self, property: &'static str, reason: String) {
        self.refusals.extend(self.styles.refusal(property, reason));
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
        Value::Calc(_) => Err(format!(
            "is a calc() with a % term, which only layout can resolve; it {}",
            later("F2c S4")
        )),
        _ => Err(shape("a length or percentage")),
    }
}

/// The keyword, or a refusal naming the value.
pub(crate) fn keyword(value: &Value) -> Result<&str, String> {
    value.as_keyword().ok_or_else(|| shape("a keyword"))
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

/// The reason for a computed value of a shape this crate does not read
/// as `expected`: vapors-css and vapors-elements disagree about the
/// property's computed form, which is a bug in one of them.
pub(crate) fn shape(expected: &str) -> String {
    format!(
        "is computed in a form vapors-elements does not read as {expected}; vapors-css and vapors-elements disagree about this property's computed value"
    )
}
