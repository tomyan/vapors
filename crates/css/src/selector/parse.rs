//! Selector lists from a rule prelude's component values.

use super::{Combinator, Compound, Selector, attribute, pseudo};
use crate::ast::ComponentValue;
use crate::token::{Token, TokenKind};
use crate::{Error, Position, Result};

/// A cursor over the prelude.
pub(super) struct Items<'a> {
    values: &'a [ComponentValue],
    index: usize,
    end: Position,
}

impl<'a> Items<'a> {
    pub(super) fn new(values: &'a [ComponentValue], end: Position) -> Self {
        Self {
            values,
            index: 0,
            end,
        }
    }

    pub(super) fn peek(&self) -> Option<&'a ComponentValue> {
        self.values.get(self.index)
    }

    pub(super) fn next(&mut self) -> Option<&'a ComponentValue> {
        let v = self.values.get(self.index)?;
        self.index += 1;
        Some(v)
    }

    pub(super) fn position(&self) -> Position {
        self.peek().map_or(self.end, ComponentValue::position)
    }

    pub(super) fn error(&self, message: impl Into<String>) -> Error {
        Error::new(self.position(), message)
    }

    pub(super) fn skip_whitespace(&mut self) -> bool {
        let mut skipped = false;
        while self.peek().is_some_and(ComponentValue::is_whitespace) {
            self.index += 1;
            skipped = true;
        }
        skipped
    }

    pub(super) fn eat_delim(&mut self, c: char) -> bool {
        if matches!(self.peek(), Some(ComponentValue::Token(Token { kind: TokenKind::Delim(d), .. })) if *d == c)
        {
            self.index += 1;
            return true;
        }
        false
    }
}

/// `a, b > c, ...`
pub(super) fn selector_list(values: &[ComponentValue], at: Position) -> Result<Vec<Selector>> {
    let end = values.last().map_or(at, ComponentValue::position);
    let mut items = Items::new(values, end);
    let mut list = Vec::new();
    loop {
        items.skip_whitespace();
        list.push(complex(&mut items)?);
        items.skip_whitespace();
        match items.next() {
            None => return Ok(list),
            Some(ComponentValue::Token(Token {
                kind: TokenKind::Comma,
                ..
            })) => {}
            Some(other) => {
                return Err(Error::new(
                    other.position(),
                    "expected \",\" or the end of the selector",
                ));
            }
        }
    }
}

/// One complex selector up to a comma or the end.
pub(super) fn complex(items: &mut Items<'_>) -> Result<Selector> {
    let mut sel = Selector {
        compounds: vec![compound(items)?],
        combinators: Vec::new(),
    };
    while let Some(comb) = combinator(items)? {
        sel.combinators.push(comb);
        sel.compounds.push(compound(items)?);
    }
    Ok(sel)
}

/// The combinator before the next compound, or `None` at a comma or end.
pub(super) fn combinator(items: &mut Items<'_>) -> Result<Option<Combinator>> {
    let had_space = items.skip_whitespace();
    let explicit = explicit_combinator(items);
    if explicit.is_some() {
        items.skip_whitespace();
        return Ok(explicit);
    }
    match items.peek() {
        None
        | Some(ComponentValue::Token(Token {
            kind: TokenKind::Comma,
            ..
        })) => Ok(None),
        Some(_) if had_space => Ok(Some(Combinator::Descendant)),
        Some(v) => Err(Error::new(
            v.position(),
            "expected a combinator or \",\" between compound selectors",
        )),
    }
}

/// `>`, `+` or `~` at the cursor, consumed.
pub(super) fn explicit_combinator(items: &mut Items<'_>) -> Option<Combinator> {
    for (c, comb) in [
        ('>', Combinator::Child),
        ('+', Combinator::NextSibling),
        ('~', Combinator::SubsequentSibling),
    ] {
        if items.eat_delim(c) {
            return Some(comb);
        }
    }
    None
}

/// A compound: optional type, then ids, classes, attributes and pseudos.
fn compound(items: &mut Items<'_>) -> Result<Compound> {
    let at = items.position();
    let mut c = Compound::default();
    let mut any = true;
    match items.peek() {
        Some(ComponentValue::Token(Token {
            kind: TokenKind::Ident(tag),
            ..
        })) => {
            c.tag = Some(tag.to_ascii_lowercase());
            items.next();
        }
        Some(ComponentValue::Token(Token {
            kind: TokenKind::Delim('*'),
            ..
        })) => {
            items.next();
        }
        _ => any = false,
    }
    while subclass(items, &mut c)? {
        any = true;
    }
    if !any {
        return Err(Error::new(
            at,
            "empty selector: expected a tag, *, class, id, attribute or pseudo-class",
        ));
    }
    Ok(c)
}

/// One id, class, attribute or pseudo-class; false when none follows.
fn subclass(items: &mut Items<'_>, c: &mut Compound) -> Result<bool> {
    match items.peek() {
        Some(ComponentValue::Token(Token {
            kind: TokenKind::Hash { value, is_id },
            position,
        })) => {
            if !is_id {
                return Err(Error::new(
                    *position,
                    format!("#{value} is not a valid id selector"),
                ));
            }
            c.id = Some(value.clone());
        }
        Some(ComponentValue::Token(Token {
            kind: TokenKind::Delim('.'),
            ..
        })) => {
            items.next();
            c.classes.push(class_name(items)?);
            return Ok(true);
        }
        Some(ComponentValue::Block {
            open: TokenKind::OpenSquare,
            contents,
            position,
        }) => {
            c.attributes.push(attribute::parse(contents, *position)?);
        }
        Some(ComponentValue::Token(Token {
            kind: TokenKind::Colon,
            position,
        })) => {
            let at = *position;
            items.next();
            c.pseudo_classes.push(pseudo::pseudo_class(items, at)?);
            return Ok(true);
        }
        _ => return Ok(false),
    }
    items.next();
    Ok(true)
}

fn class_name(items: &mut Items<'_>) -> Result<String> {
    match items.next() {
        Some(ComponentValue::Token(Token {
            kind: TokenKind::Ident(name),
            ..
        })) => Ok(name.clone()),
        _ => Err(items.error("expected a class name after \".\"")),
    }
}
