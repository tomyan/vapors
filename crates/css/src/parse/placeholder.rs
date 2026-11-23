//! `::placeholder` (CSS Pseudo 4, 4.4), narrowed to its colour
//! (`docs/placeholder.md`): written only at the end of a selector, with
//! only `color`, a rule on it becomes a rule on the originating element
//! setting `-vapors-placeholder-color`. The input paints its placeholder
//! in that longhand, so the cascade, restyle and scoping are an
//! ordinary property's. Every `::placeholder` rule gains the
//! pseudo-element's (0,0,1) alike, so leaving it out orders them the
//! same; only they set the longhand. Any other placeholder style is
//! refused, naming it.

use super::component::trim_whitespace;
use crate::ast::{ComponentValue, Declaration};
use crate::property::longhands::PLACEHOLDER_COLOR;
use crate::property::{Specified, parse_declaration};
use crate::token::{Token, TokenKind};
use crate::{Error, Result};

/// `prelude` with each selector ending in `::placeholder` replaced by
/// its originating element's, commas kept where they were so a refusal
/// names the same column; and, per selector, whether it was.
pub(super) fn rewritten(prelude: &[ComponentValue]) -> (Vec<ComponentValue>, Vec<bool>) {
    let (mut out, mut lowered) = (Vec::new(), Vec::new());
    for piece in prelude.split_inclusive(is_comma) {
        let (body, comma) = match piece.split_last() {
            Some((last, body)) if is_comma(last) => (body, Some(last)),
            _ => (piece, None),
        };
        let mut body = body.to_vec();
        trim_whitespace(&mut body);
        let originating = originating(&body);
        lowered.push(originating.is_some());
        out.extend(originating.unwrap_or(body));
        out.extend(comma.cloned());
    }
    (out, lowered)
}

fn is_comma(value: &ComponentValue) -> bool {
    matches!(
        value,
        ComponentValue::Token(Token {
            kind: TokenKind::Comma,
            ..
        })
    )
}

/// The originating element's selector when `selector` ends in
/// `::placeholder`: the rest, or `*` where nothing names it (`a >
/// ::placeholder`). `None` for any other selector.
fn originating(selector: &[ComponentValue]) -> Option<Vec<ComponentValue>> {
    let [rest @ .., first, second, name] = selector else {
        return None;
    };
    let is_colon = |v: &ComponentValue| {
        matches!(
            v,
            ComponentValue::Token(Token {
                kind: TokenKind::Colon,
                ..
            })
        )
    };
    let named = matches!(name, ComponentValue::Token(Token { kind: TokenKind::Ident(n), .. }) if n.eq_ignore_ascii_case("placeholder"));
    if !(is_colon(first) && is_colon(second) && named) {
        return None;
    }
    let mut rest = rest.to_vec();
    if rest.last().is_none_or(ends_compound_list) {
        rest.push(ComponentValue::Token(Token {
            kind: TokenKind::Delim('*'),
            position: first.position(),
        }));
    }
    Some(rest)
}

/// Whether a selector ending in `last` needs a compound after it.
fn ends_compound_list(last: &ComponentValue) -> bool {
    last.is_whitespace()
        || matches!(
            last,
            ComponentValue::Token(Token {
                kind: TokenKind::Delim('>' | '+' | '~'),
                ..
            })
        )
}

/// A `::placeholder` rule's declarations as the originating element's:
/// each `color` as `-vapors-placeholder-color`.
///
/// # Errors
/// Any other property, naming it; `inherit`, `initial` or `unset`,
/// which would mean the input's longhand, not the placeholder's colour.
pub(super) fn lowered(declarations: &[Declaration]) -> Result<Vec<Declaration>> {
    declarations.iter().map(lower).collect()
}

fn lower(declaration: &Declaration) -> Result<Declaration> {
    let at = declaration.position;
    if declaration.name != "color" {
        return Err(Error::new(
            at,
            format!(
                "`{}` in a ::placeholder rule is refused: vapors styles a placeholder's colour only, so write only `color` there (docs/placeholder.md)",
                declaration.name
            ),
        ));
    }
    let name = PLACEHOLDER_COLOR.name();
    let longhands = parse_declaration(name, &declaration.value, at)?;
    if longhands.iter().any(|l| {
        matches!(
            l.value,
            Specified::Inherit | Specified::Initial | Specified::Unset
        )
    }) {
        return Err(Error::new(
            at,
            format!(
                "`color: {}` in a ::placeholder rule is refused: it would name the input's colour, not the placeholder's default; write a colour",
                declaration.value_text()
            ),
        ));
    }
    Ok(Declaration {
        name: name.to_owned(),
        longhands,
        ..declaration.clone()
    })
}
