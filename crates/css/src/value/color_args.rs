//! The numeric arguments of a colour function: legacy comma form or the
//! modern space form with `/ alpha`; numbers, percentages, angles, `none`.

use crate::ast::ComponentValue;
use crate::token::{Token, TokenKind};
use crate::{Error, Position, Result};

/// One numeric argument of a colour function.
#[derive(Clone, Copy)]
pub(super) struct Component {
    number: f64,
    percent: bool,
    angle_unit: Option<&'static str>,
}

impl Component {
    /// Percentages map 100% to `scale`; numbers pass through.
    pub(super) fn scaled(self, scale: f64) -> f64 {
        if self.percent {
            self.number / 100.0 * scale
        } else {
            self.number
        }
    }

    /// A hue in degrees.
    pub(super) fn angle(self) -> f64 {
        match self.angle_unit {
            Some("rad") => self.number.to_degrees(),
            Some("grad") => self.number * 0.9,
            Some("turn") => self.number * 360.0,
            _ => self.number,
        }
    }
}

/// Three components and an alpha, from the legacy comma form or the
/// modern space form with `/ alpha`.
pub(super) fn arguments(args: &[ComponentValue], at: Position) -> Result<([Component; 3], f64)> {
    let mut components = Vec::new();
    let mut alpha = 1.0;
    let mut after_slash = false;
    for v in args.iter().filter(|v| !v.is_whitespace()) {
        match v {
            ComponentValue::Token(Token {
                kind: TokenKind::Comma,
                ..
            }) => {}
            ComponentValue::Token(Token {
                kind: TokenKind::Delim('/'),
                ..
            }) => after_slash = true,
            other if after_slash => alpha = component(other)?.scaled(1.0),
            other => components.push(component(other)?),
        }
    }
    if components.len() == 4 {
        alpha = components.pop().expect("four").scaled(1.0);
    }
    let [a, b, c] = components.as_slice() else {
        return Err(Error::new(
            at,
            format!(
                "a colour function takes three components and an optional alpha, found {}",
                components.len()
            ),
        ));
    };
    Ok(([*a, *b, *c], alpha))
}

fn component(value: &ComponentValue) -> Result<Component> {
    let plain = |number, percent| Component {
        number,
        percent,
        angle_unit: None,
    };
    match value {
        ComponentValue::Token(Token {
            kind: TokenKind::Number { value, .. },
            ..
        }) => Ok(plain(*value, false)),
        ComponentValue::Token(Token {
            kind: TokenKind::Percentage(p),
            ..
        }) => Ok(plain(*p, true)),
        ComponentValue::Token(Token {
            kind: TokenKind::Ident(n),
            ..
        }) if n.eq_ignore_ascii_case("none") => Ok(plain(0.0, false)),
        ComponentValue::Token(Token {
            kind: TokenKind::Dimension { value, unit },
            position,
        }) => {
            let angle_unit = ["deg", "rad", "grad", "turn"]
                .into_iter()
                .find(|u| *u == unit)
                .ok_or_else(|| {
                    Error::new(
                        *position,
                        format!("{unit} is not an angle unit; use deg, rad, grad or turn"),
                    )
                })?;
            Ok(Component {
                number: *value,
                percent: false,
                angle_unit: Some(angle_unit),
            })
        }
        other => Err(Error::new(
            other.position(),
            "colour components are numbers, percentages, angles or none",
        )),
    }
}
