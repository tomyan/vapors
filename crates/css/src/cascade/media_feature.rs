//! One media or container feature: `(name: value)`, `(name)`, or the
//! range syntax `(name >= value)` and `(lo < name <= hi)`.

use super::Environment;
use crate::ast::ComponentValue;
use crate::token::{Token, TokenKind};
use crate::value::{ColorScheme, Context, Length};
use crate::{Error, Position, Result};

/// A parsed feature.
#[derive(Clone, Debug)]
pub(crate) enum Feature {
    /// `width` or `height` compared to a value: (which, op, px).
    Size(SizeAxis, Vec<(Comparison, f64)>),
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
    fn holds(self, actual: f64, wanted: f64) -> bool {
        match self {
            Self::Less => actual < wanted,
            Self::LessEq => actual <= wanted,
            Self::Eq => (actual - wanted).abs() < f64::EPSILON,
            Self::GreaterEq => actual >= wanted,
            Self::Greater => actual > wanted,
        }
    }
}

/// What size features compare against: the viewport or a container.
pub(crate) struct Sizes {
    width: f64,
    height: f64,
    resolution: f64,
    color_scheme: ColorScheme,
    reduced_motion: bool,
}

impl Sizes {
    pub(crate) fn viewport(env: &Environment) -> Self {
        Self::container(env.viewport_width, env.viewport_height, env)
    }

    #[allow(clippy::cast_possible_truncation)] // f64 holds any f32 exactly
    pub(crate) fn container(
        width: impl Into<f64>,
        height: impl Into<f64>,
        env: &Environment,
    ) -> Self {
        Self {
            width: width.into(),
            height: height.into(),
            resolution: env.resolution,
            color_scheme: env.color_scheme,
            reduced_motion: env.reduced_motion,
        }
    }
}

impl Feature {
    pub(crate) fn holds(&self, sizes: &Sizes) -> bool {
        match self {
            Self::Size(axis, tests) => {
                let actual = match axis {
                    SizeAxis::Width => sizes.width,
                    SizeAxis::Height => sizes.height,
                };
                tests.iter().all(|(op, v)| op.holds(actual, *v))
            }
            Self::Resolution(tests) => tests.iter().all(|(op, v)| op.holds(sizes.resolution, *v)),
            Self::ColorScheme(s) => sizes.color_scheme == *s,
            Self::ReducedMotion(r) => sizes.reduced_motion == *r,
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
                vec![(Comparison::Greater, 0.0)],
            )),
            "height" => Ok(Feature::Size(
                SizeAxis::Height,
                vec![(Comparison::Greater, 0.0)],
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
    let single = || {
        value
            .first()
            .copied()
            .filter(|_| value.len() == 1)
            .ok_or_else(|| refuse("expected one value after :"))
    };
    let (base, op) = match name.strip_prefix("min-") {
        Some(base) => (base, Comparison::GreaterEq),
        None => match name.strip_prefix("max-") {
            Some(base) => (base, Comparison::LessEq),
            None => (name, Comparison::Eq),
        },
    };
    match base {
        "width" => Ok(Feature::Size(
            SizeAxis::Width,
            vec![(op, length_px(single()?, refuse)?)],
        )),
        "height" => Ok(Feature::Size(
            SizeAxis::Height,
            vec![(op, length_px(single()?, refuse)?)],
        )),
        "resolution" => Ok(Feature::Resolution(vec![(
            op,
            resolution(single()?, refuse)?,
        )])),
        "prefers-color-scheme" if name == base => match ident(Some(single()?)).as_deref() {
            Some("light") => Ok(Feature::ColorScheme(ColorScheme::Light)),
            Some("dark") => Ok(Feature::ColorScheme(ColorScheme::Dark)),
            _ => Err(refuse("prefers-color-scheme takes light or dark")),
        },
        "prefers-reduced-motion" if name == base => match ident(Some(single()?)).as_deref() {
            Some("reduce") => Ok(Feature::ReducedMotion(true)),
            Some("no-preference") => Ok(Feature::ReducedMotion(false)),
            _ => Err(refuse(
                "prefers-reduced-motion takes reduce or no-preference",
            )),
        },
        _ => Err(refuse(
            "unknown feature; supported: width, height, resolution (with min-/max-), prefers-color-scheme, prefers-reduced-motion",
        )),
    }
}

/// A length in px; relative units resolve against the root defaults.
pub(super) fn length_px(v: &ComponentValue, refuse: &dyn Fn(&str) -> Error) -> Result<f64> {
    let length = Length::parse(v).map_err(|e| refuse(&e.message))?;
    let ctx = Context {
        font_size: 16.0,
        root_font_size: 16.0,
        viewport_width: 0.0,
        viewport_height: 0.0,
        zero_advance: 8.0,
        percent_base: None,
    };
    length.to_px(&ctx).map_err(|e| refuse(&e.to_string()))
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
