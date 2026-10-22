//! Media queries (Media Queries Level 4): `[not|only]? type [and cond]*`
//! or a condition tree of `and`, `or`, `not` over features; a comma
//! separates alternatives. `@container` uses the same condition tree.

use super::Environment;
use super::media_eval::Sizes;
use super::media_feature::{self, Feature};
use super::media_items::Items;
use crate::ast::ComponentValue;
use crate::parse::component::split_commas;
use crate::token::TokenKind;
use crate::{Error, Position, Result};

/// `a, b, c`: matches when any query does.
#[derive(Clone, Debug)]
pub(crate) struct MediaQueryList(Vec<MediaQuery>);

#[derive(Clone, Debug)]
struct MediaQuery {
    negated: bool,
    media_type: MediaType,
    condition: Option<ConditionTree>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MediaType {
    All,
    Screen,
    Print,
}

/// `and`, `or`, `not` over features.
#[derive(Clone, Debug)]
pub(crate) enum ConditionTree {
    Feature(Feature),
    Not(Box<ConditionTree>),
    And(Vec<ConditionTree>),
    Or(Vec<ConditionTree>),
}

impl MediaQueryList {
    pub(crate) fn matches(&self, env: &Environment) -> bool {
        self.0.iter().any(|q| q.matches(env))
    }
}

impl MediaQuery {
    fn matches(&self, env: &Environment) -> bool {
        let type_ok = self.media_type != MediaType::Print;
        let sizes = Sizes::viewport(env);
        let cond_ok = self.condition.as_ref().is_none_or(|c| c.holds(&sizes));
        (type_ok && cond_ok) != self.negated
    }
}

impl ConditionTree {
    pub(crate) fn holds(&self, sizes: &Sizes) -> bool {
        match self {
            Self::Feature(f) => f.holds(sizes),
            Self::Not(inner) => !inner.holds(sizes),
            Self::And(items) => items.iter().all(|i| i.holds(sizes)),
            Self::Or(items) => items.iter().any(|i| i.holds(sizes)),
        }
    }
}

/// Parse a `@media` prelude.
pub(crate) fn parse_query_list(values: &[ComponentValue]) -> Result<MediaQueryList> {
    let at = values
        .first()
        .map_or(Position::default(), ComponentValue::position);
    let mut queries = Vec::new();
    for part in split_commas(values) {
        let mut items = Items::new(&part, at);
        queries.push(query(&mut items)?);
        if items.index != items.values.len() {
            return Err(Error::new(
                at,
                format!("@media: unexpected input at {}", items.position()),
            ));
        }
    }
    Ok(MediaQueryList(queries))
}

/// Parse a `@container` prelude: a condition tree only.
pub(crate) fn parse_container(values: &[ComponentValue]) -> Result<ConditionTree> {
    let at = values
        .first()
        .map_or(Position::default(), ComponentValue::position);
    let mut items = Items::new(values, at);
    let tree = condition_list(&mut items, "@container")?;
    if items.index != items.values.len() {
        return Err(Error::new(
            at,
            "@container: unexpected input after the condition",
        ));
    }
    Ok(tree)
}

fn query(items: &mut Items<'_>) -> Result<MediaQuery> {
    let negated = items.eat_ident("not");
    let only = !negated && items.eat_ident("only");
    let media_type = match items.peek_ident().as_deref() {
        Some("all") => Some(MediaType::All),
        Some("screen") => Some(MediaType::Screen),
        Some("print") => Some(MediaType::Print),
        Some(word) if word != "not" && !word.starts_with('(') => {
            return Err(items.error(format!(
                "@media: {word:?} is not a media type; use screen, print or all"
            )));
        }
        _ => None,
    };
    if let Some(media_type) = media_type {
        items.index += 1;
        let condition = if items.eat_ident("and") {
            Some(condition_list(items, "@media")?)
        } else {
            None
        };
        return Ok(MediaQuery {
            negated,
            media_type,
            condition,
        });
    }
    if only {
        return Err(items.error("@media: `only` needs a media type (only screen, only all)"));
    }
    let condition = condition_list(items, "@media")?;
    Ok(MediaQuery {
        negated,
        media_type: MediaType::All,
        condition: Some(condition),
    })
}

/// `cond [and cond]*` or `cond [or cond]*`.
fn condition_list(items: &mut Items<'_>, what: &str) -> Result<ConditionTree> {
    let first = condition(items, what)?;
    let joiner = match items.peek_ident().as_deref() {
        Some("and") => "and",
        Some("or") => "or",
        _ => return Ok(first),
    };
    let mut list = vec![first];
    while items.eat_ident(joiner) {
        list.push(condition(items, what)?);
    }
    if items
        .peek_ident()
        .as_deref()
        .is_some_and(|w| w == "and" || w == "or")
    {
        return Err(items.error(format!(
            "{what}: do not mix `and` and `or` without parentheses"
        )));
    }
    Ok(if joiner == "and" {
        ConditionTree::And(list)
    } else {
        ConditionTree::Or(list)
    })
}

/// `not cond` or `( feature | cond-list )`.
fn condition(items: &mut Items<'_>, what: &str) -> Result<ConditionTree> {
    if items.eat_ident("not") {
        return Ok(ConditionTree::Not(Box::new(condition(items, what)?)));
    }
    let Some(ComponentValue::Block {
        open: TokenKind::OpenParen,
        contents,
        position,
    }) = items.values.get(items.index)
    else {
        return Err(items.error(format!(
            "{what}: expected a parenthesised condition such as (min-width: 600px)"
        )));
    };
    items.index += 1;
    let mut inner = Items::new(contents, *position);
    if inner.peek_ident().as_deref() == Some("not")
        || matches!(inner.values.first(), Some(ComponentValue::Block { .. }))
    {
        let tree = condition_list(&mut inner, what)?;
        if inner.index != inner.values.len() {
            return Err(items.error(format!("{what}: unexpected input inside the parentheses")));
        }
        return Ok(tree);
    }
    media_feature::parse(contents, items.at, what).map(ConditionTree::Feature)
}
