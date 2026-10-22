//! Lengths: a number with a unit, resolved to px against a [`Context`].

use super::Context;
use super::calc::EvalError;
use crate::ast::ComponentValue;
use crate::token::{Token, TokenKind};
use crate::{Error, Result};

/// The units the engine has. `cell` is sumi's and refused at parse.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LengthUnit {
    /// Device-independent pixels.
    Px,
    /// The element's font size.
    Em,
    /// The root element's font size.
    Rem,
    /// A hundredth of the viewport width.
    Vw,
    /// A hundredth of the viewport height.
    Vh,
    /// The advance width of `0` in the element's font.
    Ch,
    /// A fraction of the containing block.
    Percent,
}

/// A specified length.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Length {
    /// The number before the unit.
    pub value: f64,
    /// The unit.
    pub unit: LengthUnit,
}

impl Length {
    /// Exactly `value` px.
    #[must_use]
    pub fn px(value: f64) -> Self {
        Self {
            value,
            unit: LengthUnit::Px,
        }
    }

    /// Parse a dimension, a percentage, or the unitless `0`.
    ///
    /// # Errors
    /// A unit the engine does not have, or a unitless number other than 0.
    pub fn parse(value: &ComponentValue) -> Result<Self> {
        let at = value.position();
        let ComponentValue::Token(Token { kind, .. }) = value else {
            return Err(Error::new(at, "expected a length such as 4px"));
        };
        match kind {
            TokenKind::Percentage(p) => Ok(Self {
                value: *p,
                unit: LengthUnit::Percent,
            }),
            TokenKind::Number { value: 0.0, .. } => Ok(Self::px(0.0)),
            TokenKind::Number { .. } => Err(Error::new(
                at,
                "a length needs a unit (px, em, rem, %, vw, vh or ch); only 0 may be unitless",
            )),
            TokenKind::Dimension { value, unit } => Ok(Self {
                value: *value,
                unit: unit_of(unit, at)?,
            }),
            _ => Err(Error::new(at, "expected a length such as 4px")),
        }
    }

    /// The length in px.
    ///
    /// # Errors
    /// A percentage when the context has no base for it.
    pub fn to_px(self, ctx: &Context) -> std::result::Result<f64, EvalError> {
        let scale = match self.unit {
            LengthUnit::Px => 1.0,
            LengthUnit::Em => ctx.font_size,
            LengthUnit::Rem => ctx.root_font_size,
            LengthUnit::Vw => ctx.viewport_width / 100.0,
            LengthUnit::Vh => ctx.viewport_height / 100.0,
            LengthUnit::Ch => ctx.zero_advance,
            LengthUnit::Percent => ctx.percent_base.ok_or(EvalError::NoPercentBase)? / 100.0,
        };
        Ok(self.value * scale)
    }
}

fn unit_of(unit: &str, at: crate::Position) -> Result<LengthUnit> {
    Ok(match unit {
        "px" => LengthUnit::Px,
        "em" => LengthUnit::Em,
        "rem" => LengthUnit::Rem,
        "vw" => LengthUnit::Vw,
        "vh" => LengthUnit::Vh,
        "ch" => LengthUnit::Ch,
        other => {
            return Err(Error::new(
                at,
                format!("unit {other:?} is not supported; use px, em, rem, %, vw, vh or ch"),
            ));
        }
    })
}
