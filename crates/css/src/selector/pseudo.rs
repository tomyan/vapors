//! Pseudo-classes: the names, their arguments, and the refusals.

use super::items::Items;
use super::parse::{complex, explicit_combinator, selector_list};
use super::{Combinator, Nth, PseudoClass, RelativeSelector, Selector};
use crate::ast::ComponentValue;
use crate::token::{Token, TokenKind};
use crate::{Error, Position, Result};

/// After the `:`.
pub(super) fn pseudo_class(items: &mut Items<'_>, at: Position) -> Result<PseudoClass> {
    match items.next() {
        Some(ComponentValue::Token(Token {
            kind: TokenKind::Ident(name),
            ..
        })) => simple(name, at),
        Some(ComponentValue::Function { name, args, .. }) => functional(name, args, at),
        Some(ComponentValue::Token(Token {
            kind: TokenKind::Colon,
            ..
        })) => Err(Error::new(
            at,
            "pseudo-elements (::before, ::after) are not supported; generate the content as an element",
        )),
        _ => Err(Error::new(at, "expected a pseudo-class name after \":\"")),
    }
}

fn simple(name: &str, at: Position) -> Result<PseudoClass> {
    Ok(match name.to_ascii_lowercase().as_str() {
        "hover" => PseudoClass::Hover,
        "focus" => PseudoClass::Focus,
        "focus-visible" => PseudoClass::FocusVisible,
        "focus-within" => PseudoClass::FocusWithin,
        "active" => PseudoClass::Active,
        "checked" => PseudoClass::Checked,
        "disabled" => PseudoClass::Disabled,
        "enabled" => PseudoClass::Enabled,
        "empty" => PseudoClass::Empty,
        "root" => PseudoClass::Root,
        "first-child" => PseudoClass::FirstChild,
        "last-child" => PseudoClass::LastChild,
        "only-child" => PseudoClass::OnlyChild,
        "first-of-type" => PseudoClass::FirstOfType,
        "last-of-type" => PseudoClass::LastOfType,
        "only-of-type" => PseudoClass::OnlyOfType,
        "before" | "after" => {
            return Err(Error::new(
                at,
                format!(":{name} is a pseudo-element and pseudo-elements are not supported"),
            ));
        }
        "nth-child" | "nth-last-child" | "nth-of-type" | "nth-last-of-type" | "not" | "is"
        | "where" | "has" => {
            return Err(Error::new(
                at,
                format!(":{name} needs an argument in parentheses"),
            ));
        }
        other => {
            return Err(Error::new(
                at,
                format!(
                    ":{other} is not a pseudo-class this engine knows; see docs/css.md for the list"
                ),
            ));
        }
    })
}

fn functional(name: &str, args: &[ComponentValue], at: Position) -> Result<PseudoClass> {
    let lower = name.to_ascii_lowercase();
    let nth = |make: fn(Nth) -> PseudoClass| -> Result<PseudoClass> {
        Nth::from_components(args)
            .map(make)
            .map_err(|e| Error::new(at, format!(":{lower}(): {e}")))
    };
    let list = || -> Result<Vec<Selector>> {
        if args.iter().all(ComponentValue::is_whitespace) {
            return Err(Error::new(
                at,
                format!(":{lower}() needs at least one selector"),
            ));
        }
        selector_list(args, at)
    };
    match lower.as_str() {
        "nth-child" => nth(PseudoClass::NthChild),
        "nth-last-child" => nth(PseudoClass::NthLastChild),
        "nth-of-type" => nth(PseudoClass::NthOfType),
        "nth-last-of-type" => nth(PseudoClass::NthLastOfType),
        "not" => list().map(PseudoClass::Not),
        "is" => list().map(PseudoClass::Is),
        "where" => list().map(PseudoClass::Where),
        "has" => relative_list(args, at).map(PseudoClass::Has),
        other => Err(Error::new(
            at,
            format!(
                ":{other}() is not a pseudo-class this engine knows; see docs/css.md for the list"
            ),
        )),
    }
}

/// `:has()` arguments: `[combinator] selector`, comma-separated. A nested
/// `:has()` is refused as the specification requires.
fn relative_list(args: &[ComponentValue], at: Position) -> Result<Vec<RelativeSelector>> {
    refuse_nested_has(args)?;
    let end = args.last().map_or(at, ComponentValue::position);
    let mut items = Items::new(args, end);
    let mut list = Vec::new();
    loop {
        while items.peek().is_some_and(ComponentValue::is_whitespace) {
            items.next();
        }
        let combinator = explicit_combinator(&mut items).unwrap_or(Combinator::Descendant);
        while items.peek().is_some_and(ComponentValue::is_whitespace) {
            items.next();
        }
        let selector = complex(&mut items)?;
        list.push(RelativeSelector {
            combinator,
            selector,
        });
        while items.peek().is_some_and(ComponentValue::is_whitespace) {
            items.next();
        }
        match items.next() {
            None => return Ok(list),
            Some(ComponentValue::Token(Token {
                kind: TokenKind::Comma,
                ..
            })) => {}
            Some(v) => {
                return Err(Error::new(
                    v.position(),
                    "expected \",\" or \")\" in :has()",
                ));
            }
        }
    }
}

/// `:has()` may not nest at any depth; refuse at the inner colon.
fn refuse_nested_has(args: &[ComponentValue]) -> Result<()> {
    for (i, v) in args.iter().enumerate() {
        if let ComponentValue::Function {
            name, args: inner, ..
        } = v
        {
            let after_colon = i > 0
                && matches!(
                    &args[i - 1],
                    ComponentValue::Token(Token {
                        kind: TokenKind::Colon,
                        ..
                    })
                );
            if after_colon && name.eq_ignore_ascii_case("has") {
                return Err(Error::new(
                    args[i - 1].position(),
                    ":has() inside :has() is not allowed",
                ));
            }
            refuse_nested_has(inner)?;
        }
        if let ComponentValue::Block { contents, .. } = v {
            refuse_nested_has(contents)?;
        }
    }
    Ok(())
}
