//! Values: lengths, colours, `calc()` and `var()`, and the context that
//! resolves them to computed values.

mod calc;
mod calc_eval;
mod calc_items;
mod calc_parse;
mod color;
mod color_args;
mod color_parse;
mod color_space;
mod length;
mod named_colors;
mod typed;
mod typed_calc;
mod var;

pub use calc::{Calc, CalcKind, EvalError};
pub use calc_parse::is_math_function;
pub use color::{Color, ColorScheme};
pub use length::{Length, LengthUnit};
pub use named_colors::NAMED_COLORS;
pub use typed::{Time, TimeUnit, Value};
pub use var::{VarError, substitute_vars};

use crate::Result;
use crate::ast::ComponentValue;

/// What computed-value resolution needs from the caller: the element's
/// font size for `em`, the root's for `rem`, the viewport for `vw`/`vh`,
/// the advance of `0` in the element's font for `ch`, and the containing
/// block size for `%` where the property has one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Context {
    /// The element's computed font size in px.
    pub font_size: f64,
    /// The root element's computed font size in px.
    pub root_font_size: f64,
    /// Viewport width in px.
    pub viewport_width: f64,
    /// Viewport height in px.
    pub viewport_height: f64,
    /// The advance width of `0` in the element's font, in px.
    pub zero_advance: f64,
    /// What percentages resolve against; `None` when the property has no
    /// containing-block size yet, which makes a percentage unresolvable.
    pub percent_base: Option<f64>,
}

/// Parse a colour from component values.
///
/// # Errors
/// Anything that is not a CSS Color Level 4 colour.
pub fn parse_color(values: &[ComponentValue]) -> Result<Color> {
    color_parse::parse(values)
}

/// Parse a math function from one component value.
///
/// # Errors
/// Malformed syntax, a type mismatch between operands, or division by a
/// literal zero.
pub fn parse_calc(value: &ComponentValue) -> Result<Calc> {
    calc_parse::parse(value)
}

/// Parse a colour from text, for tests and tools.
///
/// # Errors
/// Anything that is not a CSS Color Level 4 colour.
pub fn parse_color_text(text: &str) -> Result<Color> {
    let values = crate::parse::component::values_of_text(text)?;
    color_parse::parse(&values)
}

/// Parse a `calc()`, `min()`, `max()` or `clamp()` from text.
///
/// # Errors
/// Malformed syntax, a type mismatch between operands, or division by a
/// literal zero.
pub fn parse_calc_text(text: &str) -> Result<Calc> {
    let values = crate::parse::component::values_of_text(text)?;
    match values.as_slice() {
        [v] => calc_parse::parse(v),
        _ => Err(crate::Error::new(
            crate::Position { line: 1, column: 1 },
            "expected one math function",
        )),
    }
}

/// Parse a length from text.
///
/// # Errors
/// A unit the engine does not have, or a unitless number other than 0.
pub fn parse_length_text(text: &str) -> Result<Length> {
    let values = crate::parse::component::values_of_text(text)?;
    match values.as_slice() {
        [v] => Length::parse(v),
        _ => Err(crate::Error::new(
            crate::Position { line: 1, column: 1 },
            "expected one length",
        )),
    }
}
