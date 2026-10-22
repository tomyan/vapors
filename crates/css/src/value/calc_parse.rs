//! Parsing `calc()` and the comparison functions from component values,
//! with the type checks of CSS Values Level 4 section 10.

use super::calc::{Calc, CalcKind};
use super::calc_items::Items;
use super::length::Length;
use crate::ast::ComponentValue;
use crate::parse::component::split_commas;
use crate::token::{Token, TokenKind};
use crate::{Error, Position, Result};

/// Whether this component value is one of the math functions.
#[must_use]
pub fn is_math_function(value: &ComponentValue) -> bool {
    matches!(value, ComponentValue::Function { name, .. }
        if ["calc", "min", "max", "clamp"].contains(&name.to_ascii_lowercase().as_str()))
}

/// Parse a math function.
pub(super) fn parse(value: &ComponentValue) -> Result<Calc> {
    let ComponentValue::Function {
        name,
        args,
        position,
    } = value
    else {
        return Err(Error::new(
            value.position(),
            "expected calc(), min(), max() or clamp()",
        ));
    };
    let at = *position;
    let parts = split_commas(args);
    let mut exprs = Vec::new();
    for part in &parts {
        exprs.push(sum(part, at)?);
    }
    match (name.to_ascii_lowercase().as_str(), exprs.len()) {
        ("calc", 1) => Ok(exprs.remove(0)),
        ("min", n) if n >= 1 => same_kind(&exprs, at).map(|()| Calc::Min(exprs)),
        ("max", n) if n >= 1 => same_kind(&exprs, at).map(|()| Calc::Max(exprs)),
        ("clamp", 3) => {
            same_kind(&exprs, at)?;
            let [a, b, c]: [Calc; 3] = exprs.try_into().expect("three checked");
            Ok(Calc::Clamp(Box::new([a, b, c])))
        }
        ("calc", _) => Err(Error::new(at, "calc() takes one expression")),
        ("clamp", n) => Err(Error::new(
            at,
            format!("clamp() takes three arguments, found {n}"),
        )),
        (other, _) => Err(Error::new(
            at,
            format!("{other}() is not a math function; use calc, min, max or clamp"),
        )),
    }
}

fn same_kind(exprs: &[Calc], at: Position) -> Result<()> {
    let first = exprs[0].kind();
    for e in &exprs[1..] {
        if first.sum(e.kind()).is_none() {
            return Err(Error::new(
                at,
                "arguments mix a number with a length; every argument must have the same type",
            ));
        }
    }
    Ok(())
}

fn sum(values: &[ComponentValue], at: Position) -> Result<Calc> {
    let mut items = Items::new(values, at);
    let expr = sum_items(&mut items)?;
    items.skip_whitespace();
    if items.peek().is_some() {
        return Err(items.error("unexpected input after the expression"));
    }
    Ok(expr)
}

fn sum_items(items: &mut Items<'_>) -> Result<Calc> {
    let mut terms = vec![product(items)?];
    loop {
        let spaced = items.skip_whitespace();
        let Some(sign) = items.peek_delim().filter(|c| matches!(c, '+' | '-')) else {
            break;
        };
        if !spaced {
            return Err(items.error(format!("put spaces around {sign} in calc()")));
        }
        items.next();
        if !items.skip_whitespace() {
            return Err(items.error(format!("put spaces around {sign} in calc()")));
        }
        let term = product(items)?;
        let kind = terms[0].kind().sum(term.kind());
        if kind.is_none() {
            return Err(items.error(
                "cannot add a number to a length; both sides of + or - must have the same type",
            ));
        }
        terms.push(if sign == '-' {
            Calc::Negate(Box::new(term))
        } else {
            term
        });
    }
    Ok(if terms.len() == 1 {
        terms.remove(0)
    } else {
        Calc::Sum(terms)
    })
}

fn product(items: &mut Items<'_>) -> Result<Calc> {
    let mut factors = vec![factor(items)?];
    let mut kind = factors[0].kind();
    loop {
        let save = items.index;
        items.skip_whitespace();
        let Some(op) = items.peek_delim().filter(|c| matches!(c, '*' | '/')) else {
            items.index = save;
            break;
        };
        items.next();
        items.skip_whitespace();
        let rhs = factor(items)?;
        if op == '/' {
            if rhs.kind() != CalcKind::Number {
                return Err(items.error("the divisor in calc() must be a number"));
            }
            if rhs == Calc::Number(0.0) {
                return Err(items.error("division by zero in calc()"));
            }
            factors.push(Calc::Invert(Box::new(rhs)));
        } else {
            // The running product's kind, not the first factor's: a
            // product of two lengths anywhere in the chain is refused.
            kind = kind.product(rhs.kind()).ok_or_else(|| {
                items.error("cannot multiply two lengths; the other factors of * must be numbers")
            })?;
            factors.push(rhs);
        }
    }
    Ok(if factors.len() == 1 {
        factors.remove(0)
    } else {
        Calc::Product(factors)
    })
}

fn factor(items: &mut Items<'_>) -> Result<Calc> {
    items.skip_whitespace();
    let Some(value) = items.next() else {
        return Err(items.error("expected a number, a length or ( in calc()"));
    };
    match value {
        ComponentValue::Block {
            open: TokenKind::OpenParen,
            contents,
            position,
        } => sum(contents, *position),
        ComponentValue::Function { .. } => parse(value),
        ComponentValue::Token(Token {
            kind: TokenKind::Number { value: n, .. },
            ..
        }) => Ok(Calc::Number(*n)),
        ComponentValue::Token(Token {
            kind: TokenKind::Dimension { .. } | TokenKind::Percentage(_),
            ..
        }) => Length::parse(value).map(Calc::Length),
        other => Err(Error::new(
            other.position(),
            "expected a number, a length or ( in calc()",
        )),
    }
}
