//! Timing functions (Easing Level 1): cubic Bézier curves solved for x
//! by Newton iteration, and step functions with every jump position.

use std::fmt;

use super::timing_curve;
use crate::value::Value;

/// Where a step function jumps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepPosition {
    /// `start` / `jump-start`: the first jump happens at 0.
    Start,
    /// `end` / `jump-end`: the last jump happens at 1 (the default).
    End,
    /// `jump-none`: no jump at either end; `count` steps span 0 to 1.
    JumpNone,
    /// `jump-both`: a jump at both ends.
    JumpBoth,
}

/// An easing curve.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TimingFunction {
    /// `cubic-bezier(x1, y1, x2, y2)`.
    CubicBezier {
        /// First control point x, in 0..=1.
        x1: f64,
        /// First control point y.
        y1: f64,
        /// Second control point x, in 0..=1.
        x2: f64,
        /// Second control point y.
        y2: f64,
    },
    /// `steps(count, position)`.
    Steps {
        /// Number of steps, at least 1.
        count: u32,
        /// Jump position.
        position: StepPosition,
    },
}

/// Why a timing function text was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimingError(String);

impl fmt::Display for TimingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} is not a timing function", self.0)
    }
}

impl std::error::Error for TimingError {}

impl TimingFunction {
    /// `linear`
    pub const LINEAR: Self = Self::bezier(0.0, 0.0, 1.0, 1.0);
    /// `ease`
    pub const EASE: Self = Self::bezier(0.25, 0.1, 0.25, 1.0);
    /// `ease-in`
    pub const EASE_IN: Self = Self::bezier(0.42, 0.0, 1.0, 1.0);
    /// `ease-out`
    pub const EASE_OUT: Self = Self::bezier(0.0, 0.0, 0.58, 1.0);
    /// `ease-in-out`
    pub const EASE_IN_OUT: Self = Self::bezier(0.42, 0.0, 0.58, 1.0);

    const fn bezier(x1: f64, y1: f64, x2: f64, y2: f64) -> Self {
        Self::CubicBezier { x1, y1, x2, y2 }
    }

    /// Parse from text.
    ///
    /// # Errors
    /// Anything that is not a named curve, `cubic-bezier()` or `steps()`.
    pub fn parse(text: &str) -> Result<Self, TimingError> {
        let refuse = || TimingError(format!("{text:?}"));
        let values = crate::parse::component::values_of_text(text).map_err(|_| refuse())?;
        let grammar = crate::syntax::parse_grammar("<easing-function>");
        let value = crate::syntax::match_value(&grammar, &values).ok_or_else(refuse)?;
        Self::from_value(&value).ok_or_else(refuse)
    }

    /// From a value the `<easing-function>` grammar produced.
    #[must_use]
    pub fn from_value(value: &Value) -> Option<Self> {
        match value {
            Value::Keyword(k) => Self::from_keyword(k),
            Value::Function(name, args) if name == "cubic-bezier" => match args.as_slice() {
                [
                    Value::Number(x1),
                    Value::Number(y1),
                    Value::Number(x2),
                    Value::Number(y2),
                ] => Some(Self::bezier(*x1, *y1, *x2, *y2)),
                _ => None,
            },
            Value::Function(name, args) if name == "steps" => Self::from_steps(args),
            _ => None,
        }
    }

    fn from_keyword(k: &str) -> Option<Self> {
        Some(match k {
            "linear" => Self::LINEAR,
            "ease" => Self::EASE,
            "ease-in" => Self::EASE_IN,
            "ease-out" => Self::EASE_OUT,
            "ease-in-out" => Self::EASE_IN_OUT,
            "step-start" => Self::Steps {
                count: 1,
                position: StepPosition::Start,
            },
            "step-end" => Self::Steps {
                count: 1,
                position: StepPosition::End,
            },
            _ => return None,
        })
    }

    fn from_steps(args: &[Value]) -> Option<Self> {
        let count = match args.first() {
            Some(Value::Integer(n)) if *n >= 1 => u32::try_from(*n).ok()?,
            _ => return None,
        };
        let position = match args.get(1).and_then(Value::as_keyword) {
            None | Some("end" | "jump-end") => StepPosition::End,
            Some("start" | "jump-start") => StepPosition::Start,
            Some("jump-none") => StepPosition::JumpNone,
            Some("jump-both") => StepPosition::JumpBoth,
            Some(_) => return None,
        };
        Some(Self::Steps { count, position })
    }

    /// The eased output for an input progress `t` in 0..=1.
    #[must_use]
    pub fn evaluate(&self, t: f64) -> f64 {
        if t <= 0.0 {
            return 0.0;
        }
        if t >= 1.0 {
            return 1.0;
        }
        match *self {
            // Control points on the diagonal make the identity; skip the
            // solve so linear stays exact.
            Self::CubicBezier { x1, y1, x2, y2 }
                if x1.to_bits() == y1.to_bits() && x2.to_bits() == y2.to_bits() =>
            {
                t
            }
            Self::CubicBezier { x1, y1, x2, y2 } => timing_curve::bezier_y(x1, y1, x2, y2, t),
            Self::Steps { count, position } => timing_curve::steps(t, count, position),
        }
    }
}
