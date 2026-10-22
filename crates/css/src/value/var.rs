//! `var()` substitution over component values, with fallbacks and cycle
//! refusal (CSS Variables Level 1 section 3).

use std::fmt;

use crate::ast::ComponentValue;
use crate::parse::component::split_commas;
use crate::token::{Token, TokenKind};

/// Why a `var()` could not be substituted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VarError {
    /// The variable is not defined and no fallback was given.
    Unresolved(String),
    /// The variables reference each other; the chain, first repeated last.
    Cycle(Vec<String>),
    /// `var()` was given something other than a `--name`.
    BadName(String),
}

impl fmt::Display for VarError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unresolved(name) => write!(
                f,
                "var({name}) is not defined on this element or an ancestor and has no fallback"
            ),
            Self::Cycle(chain) => write!(f, "var() cycle: {}", chain.join(" -> ")),
            Self::BadName(text) => write!(
                f,
                "var({text}) must name a custom property starting with --"
            ),
        }
    }
}

impl std::error::Error for VarError {}

/// Replace every `var()` in `values` using `lookup`, recursively.
///
/// # Errors
/// An undefined variable without fallback, a cycle, or a bad name.
pub fn substitute_vars<'a>(
    values: &[ComponentValue],
    lookup: &dyn Fn(&str) -> Option<&'a [ComponentValue]>,
) -> Result<Vec<ComponentValue>, VarError> {
    let mut chain = Vec::new();
    substitute(values, lookup, &mut chain)
}

fn substitute<'a>(
    values: &[ComponentValue],
    lookup: &dyn Fn(&str) -> Option<&'a [ComponentValue]>,
    chain: &mut Vec<String>,
) -> Result<Vec<ComponentValue>, VarError> {
    let mut out = Vec::new();
    for v in values {
        match v {
            ComponentValue::Function { name, args, .. } if name.eq_ignore_ascii_case("var") => {
                out.extend(reference(args, lookup, chain)?);
            }
            ComponentValue::Function {
                name,
                args,
                position,
            } => out.push(ComponentValue::Function {
                name: name.clone(),
                args: substitute(args, lookup, chain)?,
                position: *position,
            }),
            ComponentValue::Block {
                open,
                contents,
                position,
            } => out.push(ComponentValue::Block {
                open: open.clone(),
                contents: substitute(contents, lookup, chain)?,
                position: *position,
            }),
            ComponentValue::Token(_) => out.push(v.clone()),
        }
    }
    Ok(out)
}

/// One `var(--name[, fallback])`.
fn reference<'a>(
    args: &[ComponentValue],
    lookup: &dyn Fn(&str) -> Option<&'a [ComponentValue]>,
    chain: &mut Vec<String>,
) -> Result<Vec<ComponentValue>, VarError> {
    let parts = split_commas(args);
    let name = match parts[0].as_slice() {
        [
            ComponentValue::Token(Token {
                kind: TokenKind::Ident(n),
                ..
            }),
        ] if n.starts_with("--") => n.clone(),
        _ => {
            return Err(VarError::BadName(crate::serialize::component_values(
                &parts[0],
            )));
        }
    };
    if chain.contains(&name) {
        chain.push(name);
        return Err(VarError::Cycle(std::mem::take(chain)));
    }
    if let Some(value) = lookup(&name) {
        chain.push(name.clone());
        let out = substitute(value, lookup, chain)?;
        chain.pop();
        return Ok(out);
    }
    match parts.get(1..) {
        Some([]) | None => Err(VarError::Unresolved(name)),
        Some(fallback) => {
            let joined: Vec<ComponentValue> = fallback.join(&comma());
            substitute(&joined, lookup, chain)
        }
    }
}

fn comma() -> ComponentValue {
    ComponentValue::Token(Token {
        kind: TokenKind::Comma,
        position: crate::Position::default(),
    })
}
