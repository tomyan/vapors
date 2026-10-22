//! The `An+B` microsyntax of `:nth-*()` (CSS Syntax Level 3 section 6).

use std::fmt;

use crate::ast::ComponentValue;
use crate::token::{Token, TokenKind};

/// `An+B`: matches 1-based index `i` when `i = A*n + B` for some `n >= 0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Nth {
    /// The step.
    pub a: i32,
    /// The offset.
    pub b: i32,
}

/// Why an `An+B` text was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NthError(String);

impl fmt::Display for NthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} is not An+B, odd, even or an integer", self.0)
    }
}

impl std::error::Error for NthError {}

impl Nth {
    /// Whether the 1-based `index` satisfies the expression.
    #[must_use]
    pub fn matches(self, index: i32) -> bool {
        if self.a == 0 {
            return index == self.b;
        }
        let diff = index - self.b;
        diff % self.a == 0 && diff / self.a >= 0
    }

    /// Parse `odd`, `even`, an integer, or `An+B` with whitespace allowed
    /// only around the sign before B.
    ///
    /// # Errors
    /// Anything else, quoting the text.
    pub fn parse(text: &str) -> Result<Self, NthError> {
        let refuse = || NthError(text.to_owned());
        let tokens = crate::token::tokenize(text).map_err(|_| refuse())?;
        let values: Vec<ComponentValue> = tokens.into_iter().map(ComponentValue::Token).collect();
        Self::from_components(&values).map_err(|_| refuse())
    }

    /// Parse from the argument of `:nth-child()` and friends.
    ///
    /// # Errors
    /// Anything that is not An+B, quoting the argument text.
    pub fn from_components(values: &[ComponentValue]) -> Result<Self, NthError> {
        let refuse = || NthError(crate::serialize::component_values(values).trim().to_owned());
        let kinds: Vec<&TokenKind> = values
            .iter()
            .filter(|v| !v.is_whitespace())
            .filter_map(|v| match v {
                ComponentValue::Token(Token { kind, .. }) => Some(kind),
                _ => None,
            })
            .collect();
        if kinds.len() != values.iter().filter(|v| !v.is_whitespace()).count() {
            return Err(refuse());
        }
        if kinds
            .iter()
            .any(|k| matches!(k, TokenKind::Ident(w) if w.eq_ignore_ascii_case("of")))
        {
            return Err(NthError(format!(
                "{} (`of <selector>` is not supported; use :is() on the element)",
                crate::serialize::component_values(values).trim()
            )));
        }
        from_kinds(&kinds).ok_or_else(refuse)
    }
}

fn from_kinds(kinds: &[&TokenKind]) -> Option<Nth> {
    match kinds {
        [TokenKind::Ident(w)] if w.eq_ignore_ascii_case("odd") => Some(Nth { a: 2, b: 1 }),
        [TokenKind::Ident(w)] if w.eq_ignore_ascii_case("even") => Some(Nth { a: 2, b: 0 }),
        [
            TokenKind::Number {
                value,
                integer: true,
                ..
            },
        ] => Some(Nth {
            a: 0,
            b: int(*value)?,
        }),
        // `+n`, `+n+1`: a leading plus delimiter before the ident.
        [TokenKind::Delim('+'), TokenKind::Ident(name), rest @ ..]
            if name.starts_with(['n', 'N']) =>
        {
            from_kinds(
                &std::iter::once(&&TokenKind::Ident(name.clone()))
                    .chain(rest.iter())
                    .copied()
                    .collect::<Vec<_>>(),
            )
        }
        [head, rest @ ..] => {
            let (a, b_in_head) = head_part(head)?;
            if rest.is_empty() {
                return Some(Nth {
                    a,
                    b: b_in_head.unwrap_or(0),
                });
            }
            if b_in_head.is_some() {
                return None;
            }
            Some(Nth {
                a,
                b: tail_part(rest)?,
            })
        }
        [] => None,
    }
}

/// `n`, `-n`, `+n`, `2n`, or those with a trailing `-B` glued on as the
/// tokenizer produces (`n-3`, `2n-1`). Returns A and any B found.
fn head_part(head: &TokenKind) -> Option<(i32, Option<i32>)> {
    let (a, name) = match head {
        TokenKind::Ident(name) => match name.strip_prefix('-') {
            Some(rest) => (-1, rest),
            None => (1, name.as_str()),
        },
        TokenKind::Dimension { value, unit } => (int(*value)?, unit.as_str()),
        _ => return None,
    };
    let rest = name.strip_prefix(['n', 'N'])?;
    if rest.is_empty() {
        return Some((a, None));
    }
    let digits = rest.strip_prefix('-')?;
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some((a, Some(-digits.parse::<i32>().ok()?)))
}

/// After the `n` part: a signed integer, or a sign delimiter then an
/// unsigned integer.
fn tail_part(rest: &[&TokenKind]) -> Option<i32> {
    match rest {
        [
            TokenKind::Number {
                value,
                integer: true,
                signed: true,
            },
        ] => int(*value),
        [
            TokenKind::Delim(sign),
            TokenKind::Number {
                value,
                integer: true,
                signed: false,
            },
        ] => {
            let b = int(*value)?;
            match sign {
                '+' => Some(b),
                '-' => Some(-b),
                _ => None,
            }
        }
        _ => None,
    }
}

#[allow(clippy::cast_possible_truncation)] // checked to be integral and in range
fn int(value: f64) -> Option<i32> {
    if value.fract() != 0.0 || value.abs() > f64::from(i32::MAX) {
        return None;
    }
    Some(value as i32)
}
