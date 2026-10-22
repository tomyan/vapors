//! The `calc()` expression tree and its type (CSS Values Level 4
//! section 10).

use std::fmt;

use super::Length;

/// What a math expression evaluates to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CalcKind {
    /// A plain number.
    Number,
    /// A length in px once resolved.
    Length,
    /// A percentage of the containing block.
    Percentage,
    /// A length that includes a percentage term.
    LengthPercentage,
}

impl CalcKind {
    /// The kind of `a + b`, or `None` when they cannot be added.
    pub(super) fn sum(self, other: Self) -> Option<Self> {
        use CalcKind::{Length, LengthPercentage, Number, Percentage};
        match (self, other) {
            (Number, Number) => Some(Number),
            (Number, _) | (_, Number) => None,
            (Length, Length) => Some(Length),
            (Percentage, Percentage) => Some(Percentage),
            _ => Some(LengthPercentage),
        }
    }

    /// The kind of `a * b`, or `None` when neither is a number.
    pub(super) fn product(self, other: Self) -> Option<Self> {
        match (self, other) {
            (CalcKind::Number, k) | (k, CalcKind::Number) => Some(k),
            _ => None,
        }
    }
}

/// A parsed math expression.
#[derive(Clone, Debug, PartialEq)]
pub enum Calc {
    /// A number.
    Number(f64),
    /// A length or percentage.
    Length(Length),
    /// `a + b + ...`, each term with its sign applied.
    Sum(Vec<Calc>),
    /// `a * b * ...`, each with division already folded into `Invert`.
    Product(Vec<Calc>),
    /// `-a`
    Negate(Box<Calc>),
    /// `1 / a`
    Invert(Box<Calc>),
    /// `min(a, b, ...)`
    Min(Vec<Calc>),
    /// `max(a, b, ...)`
    Max(Vec<Calc>),
    /// `clamp(min, value, max)`
    Clamp(Box<[Calc; 3]>),
}

/// Why an expression could not be evaluated.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EvalError {
    /// A percentage term, but the context has no base for percentages.
    NoPercentBase,
    /// A division whose divisor evaluated to zero.
    DivisionByZero,
}

impl fmt::Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoPercentBase => {
                write!(
                    f,
                    "a % term needs a containing block size and none was supplied"
                )
            }
            Self::DivisionByZero => write!(f, "division by zero"),
        }
    }
}

impl std::error::Error for EvalError {}

impl Calc {
    /// The expression's kind.
    ///
    /// # Panics
    /// Never for an expression the parser produced: the parser refuses
    /// operands whose kinds disagree.
    #[must_use]
    pub fn kind(&self) -> CalcKind {
        match self {
            Self::Number(_) => CalcKind::Number,
            Self::Length(l) if l.unit == super::LengthUnit::Percent => CalcKind::Percentage,
            Self::Length(_) => CalcKind::Length,
            Self::Negate(a) | Self::Invert(a) => a.kind(),
            Self::Sum(terms) | Self::Min(terms) | Self::Max(terms) => terms
                .iter()
                .map(Self::kind)
                .reduce(|a, b| a.sum(b).expect("parser checked the kinds"))
                .expect("at least one term"),
            Self::Product(factors) => factors
                .iter()
                .map(Self::kind)
                .reduce(|a, b| a.product(b).expect("parser checked the kinds"))
                .expect("at least one factor"),
            Self::Clamp(args) => args
                .iter()
                .map(Self::kind)
                .reduce(|a, b| a.sum(b).expect("parser checked the kinds"))
                .expect("three arguments"),
        }
    }
}
