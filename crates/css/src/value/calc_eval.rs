//! Evaluating a [`Calc`] against a [`Context`].

use super::Context;
use super::calc::{Calc, EvalError};

impl Calc {
    /// The expression's value: px for lengths, a plain number otherwise.
    ///
    /// # Errors
    /// A percentage without a base, or a division by zero.
    pub fn evaluate(&self, ctx: &Context) -> Result<f64, EvalError> {
        match self {
            Self::Number(n) => Ok(*n),
            Self::Length(l) => l.to_px(ctx),
            Self::Negate(a) => Ok(-a.evaluate(ctx)?),
            Self::Invert(a) => {
                let v = a.evaluate(ctx)?;
                if v == 0.0 {
                    return Err(EvalError::DivisionByZero);
                }
                Ok(1.0 / v)
            }
            Self::Sum(terms) => fold(terms, ctx, 0.0, |a, b| a + b),
            Self::Product(factors) => fold(factors, ctx, 1.0, |a, b| a * b),
            Self::Min(args) => fold(args, ctx, f64::INFINITY, f64::min),
            Self::Max(args) => fold(args, ctx, f64::NEG_INFINITY, f64::max),
            Self::Clamp(args) => {
                let [lo, v, hi] = args.as_ref();
                let (lo, v, hi) = (lo.evaluate(ctx)?, v.evaluate(ctx)?, hi.evaluate(ctx)?);
                // clamp(MIN, VAL, MAX) = max(MIN, min(VAL, MAX)): MIN wins
                // when it is above MAX (Values Level 4 section 10.3).
                Ok(lo.max(v.min(hi)))
            }
        }
    }
}

fn fold(
    terms: &[Calc],
    ctx: &Context,
    init: f64,
    op: fn(f64, f64) -> f64,
) -> Result<f64, EvalError> {
    let mut acc = init;
    for t in terms {
        acc = op(acc, t.evaluate(ctx)?);
    }
    Ok(acc)
}
