//! A declaration's value as typed longhands: CSS-wide keywords, a parsed
//! [`Value`], or component values left pending because of `var()`.

use super::{PropertyId, names, shorthand};
use crate::ast::ComponentValue;
use crate::syntax::match_value;
use crate::token::{Token, TokenKind};
use crate::value::Value;
use crate::{Error, Position, Result};

/// What a longhand was declared as.
#[derive(Clone, Debug, PartialEq)]
pub enum Specified {
    /// A value that parsed under the property's grammar.
    Value(Value),
    /// `inherit`
    Inherit,
    /// `initial`
    Initial,
    /// `unset`
    Unset,
    /// Component values containing `var()`, parsed after substitution
    /// under the declared property (a shorthand expands then).
    Pending {
        /// The property as declared, longhand or shorthand.
        property: String,
        /// The declared value.
        tokens: Vec<ComponentValue>,
    },
}

impl Specified {
    /// Canonical CSS text.
    #[must_use]
    pub fn to_css(&self) -> String {
        match self {
            Self::Value(v) => v.to_css(),
            Self::Inherit => "inherit".to_owned(),
            Self::Initial => "initial".to_owned(),
            Self::Unset => "unset".to_owned(),
            Self::Pending { tokens, .. } => crate::serialize::component_values(tokens),
        }
    }

    /// Whether `var()` substitution is still to come.
    #[must_use]
    pub fn is_pending(&self) -> bool {
        matches!(self, Self::Pending { .. })
    }
}

/// One longhand a declaration sets.
#[derive(Clone, Debug, PartialEq)]
pub struct Longhand {
    /// The property.
    pub id: PropertyId,
    /// Its value.
    pub value: Specified,
}

/// Parse a declaration's value into longhands. A custom property gives
/// none; a shorthand gives several.
///
/// # Errors
/// A value that fails the property's grammar, naming the property, the
/// value and the grammar.
///
/// # Panics
/// When `name` is not a longhand, shorthand or custom property: the
/// parser checks names first with `check_name`.
pub fn parse_declaration(
    name: &str,
    values: &[ComponentValue],
    at: Position,
) -> Result<Vec<Longhand>> {
    if name.starts_with("--") {
        return Ok(Vec::new());
    }
    let value_at = values.first().map_or(at, ComponentValue::position);
    if let Some(wide) = css_wide(values) {
        return Ok(longhand_ids(name)
            .into_iter()
            .map(|id| Longhand {
                id,
                value: wide.clone(),
            })
            .collect());
    }
    if contains_var(values) {
        let pending = Specified::Pending {
            property: name.to_owned(),
            tokens: values.to_vec(),
        };
        return Ok(longhand_ids(name)
            .into_iter()
            .map(|id| Longhand {
                id,
                value: pending.clone(),
            })
            .collect());
    }
    if shorthand::is_shorthand(name) {
        return shorthand::expand(name, values, value_at);
    }
    let id = PropertyId::from_name(name).expect("caller checked the name");
    let value = parse_longhand(id, values, value_at)?;
    if name == "transition-property" {
        check_transition_properties(&value, value_at)?;
    }
    Ok(vec![Longhand {
        id,
        value: Specified::Value(value),
    }])
}

/// Every longhand a name stands for.
fn longhand_ids(name: &str) -> Vec<PropertyId> {
    if let Some(id) = PropertyId::from_name(name) {
        return vec![id];
    }
    shorthand::longhands_of(name)
}

/// Parse one longhand's value under its grammar.
pub(super) fn parse_longhand(
    id: PropertyId,
    values: &[ComponentValue],
    at: Position,
) -> Result<Value> {
    match_value(id.grammar(), values).ok_or_else(|| refusal(id.name(), id.def().syntax, values, at))
}

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

fn css_wide(values: &[ComponentValue]) -> Option<Specified> {
    match values {
        [
            ComponentValue::Token(Token {
                kind: TokenKind::Ident(k),
                ..
            }),
        ] => match k.to_ascii_lowercase().as_str() {
            "inherit" => Some(Specified::Inherit),
            "initial" => Some(Specified::Initial),
            "unset" => Some(Specified::Unset),
            _ => None,
        },
        _ => None,
    }
}

fn contains_var(values: &[ComponentValue]) -> bool {
    values.iter().any(|v| match v {
        ComponentValue::Function { name, args, .. } => {
            name.eq_ignore_ascii_case("var") || contains_var(args)
        }
        ComponentValue::Block { contents, .. } => contains_var(contents),
        ComponentValue::Token(_) => false,
    })
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
