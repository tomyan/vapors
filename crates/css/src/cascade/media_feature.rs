//! One media or container feature: `(name: value)`, `(name)`, or the
//! range syntax `(name >= value)` and `(lo < name <= hi)`.

use crate::ast::ComponentValue;
use crate::token::{Token, TokenKind};
use crate::value::{ColorScheme, Length, LengthUnit};
use crate::{Error, Position, Result};

/// A parsed feature.
#[derive(Clone, Debug)]
pub(crate) enum Feature {
    /// `width` or `height` compared to lengths, resolved at evaluation
    /// against the environment (vw, vh, rem).
    Size(SizeAxis, Vec<(Comparison, Length)>),
    /// `resolution` compared to dppx.
    Resolution(Vec<(Comparison, f64)>),
    /// `prefers-color-scheme: light | dark`.
    ColorScheme(ColorScheme),
    /// `prefers-reduced-motion: reduce | no-preference`.
    ReducedMotion(bool),
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum SizeAxis {
    Width,
    Height,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Comparison {
    Less,
    LessEq,
    Eq,
    GreaterEq,
    Greater,
}

impl Comparison {
    pub(super) fn holds(self, actual: f64, wanted: f64) -> bool {
        match self {
            Self::Less => actual < wanted,
            Self::LessEq => actual <= wanted,
            Self::Eq => (actual - wanted).abs() < f64::EPSILON,
            Self::GreaterEq => actual >= wanted,
            Self::Greater => actual > wanted,
        }
    }
}

/// Parse the inside of the parentheses.
pub(crate) fn parse(contents: &[ComponentValue], at: Position, what: &str) -> Result<Feature> {
    let items: Vec<&ComponentValue> = contents.iter().filter(|v| !v.is_whitespace()).collect();
    let text = crate::serialize::component_values(contents);
    let refuse = |why: &str| Error::new(at, format!("{what}: ({text}): {why}"));
    if let Some(i) = items.iter().position(|v| {
        matches!(
            v,
            ComponentValue::Token(Token {
                kind: TokenKind::Colon,
                ..
            })
        )
    }) {
        let name = ident(items[..i].first().copied())
            .ok_or_else(|| refuse("expected a feature name before :"))?;
        if i != 1 {
            return Err(refuse("expected one feature name before :"));
        }
        return colon_form(&name, &items[i + 1..], &refuse);
    }
    if items.len() == 1 {
        let name = ident(Some(items[0])).ok_or_else(|| refuse("expected a feature name"))?;
        return match name.as_str() {
            "width" => Ok(Feature::Size(
                SizeAxis::Width,
                vec![(Comparison::Greater, Length::px(0.0))],
            )),
            "height" => Ok(Feature::Size(
                SizeAxis::Height,
                vec![(Comparison::Greater, Length::px(0.0))],
            )),
            "prefers-reduced-motion" => Ok(Feature::ReducedMotion(true)),
            _ => Err(refuse("this feature needs a value")),
        };
    }
    super::media_range::range_form(&items, &refuse)
}

pub(super) fn ident(v: Option<&ComponentValue>) -> Option<String> {
    match v {
        Some(ComponentValue::Token(Token {
            kind: TokenKind::Ident(s),
            ..
        })) => Some(s.to_ascii_lowercase()),
        _ => None,
    }
}

fn colon_form(
    name: &str,
    value: &[&ComponentValue],
    refuse: &dyn Fn(&str) -> Error,
) -> Result<Feature> {
    let single = value
        .first()
        .copied()
        .filter(|_| value.len() == 1)
        .ok_or_else(|| refuse("expected one value after :"))?;
    let (base, op) = match (name.strip_prefix("min-"), name.strip_prefix("max-")) {
        (Some(base), _) => (base, Comparison::GreaterEq),
        (_, Some(base)) => (base, Comparison::LessEq),
        _ => (name, Comparison::Eq),
    };
    match base {
        "width" => Ok(Feature::Size(
            SizeAxis::Width,
            vec![(op, length(single, refuse)?)],
        )),
        "height" => Ok(Feature::Size(
            SizeAxis::Height,
            vec![(op, length(single, refuse)?)],
        )),
        "resolution" => Ok(Feature::Resolution(vec![(op, resolution(single, refuse)?)])),
        _ if name == base => preference(base, single, refuse),
        _ => Err(refuse(
            "only width, height and resolution take min- and max-",
        )),
    }
}

/// `prefers-color-scheme` and `prefers-reduced-motion`.
fn preference(
    name: &str,
    value: &ComponentValue,
    refuse: &dyn Fn(&str) -> Error,
) -> Result<Feature> {
    let word = ident(Some(value));
    match (name, word.as_deref()) {
        ("prefers-color-scheme", Some("light")) => Ok(Feature::ColorScheme(ColorScheme::Light)),
        ("prefers-color-scheme", Some("dark")) => Ok(Feature::ColorScheme(ColorScheme::Dark)),
        ("prefers-color-scheme", _) => Err(refuse("prefers-color-scheme takes light or dark")),
        ("prefers-reduced-motion", Some("reduce")) => Ok(Feature::ReducedMotion(true)),
        ("prefers-reduced-motion", Some("no-preference")) => Ok(Feature::ReducedMotion(false)),
        ("prefers-reduced-motion", _) => Err(refuse(
            "prefers-reduced-motion takes reduce or no-preference",
        )),
        _ => Err(refuse(
            "unknown feature; supported: width, height, resolution (with min-/max-), prefers-color-scheme, prefers-reduced-motion",
        )),
    }
}

/// A length feature value; `%` has no meaning here.
pub(super) fn length(v: &ComponentValue, refuse: &dyn Fn(&str) -> Error) -> Result<Length> {
    let length = Length::parse(v).map_err(|e| refuse(&e.message))?;
    if length.unit == LengthUnit::Percent {
        return Err(refuse("a percentage has no meaning in a size feature"));
    }
    Ok(length)
}

/// `<number>dppx | x | dpi`.
pub(super) fn resolution(v: &ComponentValue, refuse: &dyn Fn(&str) -> Error) -> Result<f64> {
    match v {
        ComponentValue::Token(Token {
            kind: TokenKind::Dimension { value, unit },
            ..
        }) => match unit.as_str() {
            "dppx" | "x" => Ok(*value),
            "dpi" => Ok(value / 96.0),
            _ => Err(refuse("resolution takes dppx, x or dpi")),
        },
        _ => Err(refuse("resolution takes dppx, x or dpi")),
    }
}
