//! Refusal text for a value that failed its grammar, and the
//! transition-property name check.

use super::declaration::{Specified, parse_declaration};
use super::{PropertyId, names};
use crate::ast::ComponentValue;
use crate::token::{Token, TokenKind};
use crate::value::Value;
use crate::{Error, Position, Result};

/// The refusal for a value that failed a grammar.
pub(super) fn refusal(
    property: &str,
    syntax: &str,
    values: &[ComponentValue],
    at: Position,
) -> Error {
    let text = crate::serialize::component_values(values);
    let has_square = values.iter().any(|v| {
        matches!(
            v,
            ComponentValue::Block {
                open: TokenKind::OpenSquare,
                ..
            }
        )
    });
    if property.starts_with("grid-template") && has_square {
        return Error::new(
            at,
            format!(
                "{property}: grid line names ([name]) are not supported; place items by number"
            ),
        );
    }
    for v in values {
        if let ComponentValue::Token(Token {
            kind: TokenKind::Ident(k),
            ..
        }) = v
            && let Some(hint) = names::sumi_value_hint(property, &k.to_ascii_lowercase())
        {
            return Error::new(at, format!("{property}: {text}: {hint}"));
        }
    }
    Error::new(
        at,
        format!(
            "{property}: {text:?} does not match the syntax {}",
            describe_syntax(syntax)
        ),
    )
}

/// The grammar with the special types spelled out.
fn describe_syntax(syntax: &str) -> String {
    let mut out = syntax.to_owned();
    for (name, description) in [
        (
            "<transform-function>",
            "translate(x, y?) | translateX(x) | translateY(y) | scale(x, y?)",
        ),
        ("<filter-function>", "blur(<length>)"),
        ("<shadow>", "inset? <length>{2,4} <color>?"),
        (
            "<easing-function>",
            "linear | ease | ease-in | ease-out | ease-in-out | step-start | step-end | cubic-bezier(x1, y1, x2, y2) | steps(n, position?)",
        ),
        (
            "<track-size>",
            "<length-percentage> | <flex> | auto | min-content | max-content | minmax(min, max)",
        ),
        (
            "<track-list>",
            "[ <track-size> | repeat(count, <track-size>+) ]+",
        ),
        ("<font-family-name>", "<string> | <ident>+"),
        ("<opentype-tag>", "a four-character string such as \"tnum\""),
    ] {
        out = out.replace(name, &format!("[ {description} ]"));
    }
    out
}

/// Every ident in `transition-property` must be a property.
pub(super) fn check_transition_properties(value: &Value, at: Position) -> Result<()> {
    let items = match value {
        Value::CommaList(items) => items.as_slice(),
        single => std::slice::from_ref(single),
    };
    for item in items {
        if let Some(name) = item.as_keyword()
            && !["all", "none"].contains(&name)
            && !super::is_supported(name)
        {
            return Err(Error::new(
                at,
                format!("transition-property: {name:?} is not a property"),
            ));
        }
    }
    Ok(())
}

/// Substitute `var()` in a pending declaration and parse it under the
/// declared property, returning what longhand `id` gets.
///
/// # Errors
/// An unresolved variable or cycle, or a value that fails the grammar,
/// each naming the declaration at `at`.
pub fn resolve_pending<'a>(
    property: &str,
    tokens: &[ComponentValue],
    lookup: &dyn Fn(&str) -> Option<&'a [ComponentValue]>,
    id: PropertyId,
    at: Position,
) -> Result<Specified> {
    let substituted = crate::value::substitute_vars(tokens, lookup)
        .map_err(|e| Error::new(at, format!("{property}: {e}")))?;
    let longhands = parse_declaration(property, &substituted, at)
        .map_err(|e| Error::new(at, format!("after var() substitution, {}", e.message)))?;
    longhands
        .into_iter()
        .find(|l| l.id == id)
        .map(|l| l.value)
        .ok_or_else(|| Error::new(at, format!("{property} does not set {}", id.name())))
}
