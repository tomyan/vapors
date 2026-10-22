//! Stylesheet and qualified rules (sections 5.4.1 to 5.4.3).

use super::component::{split_commas, values_until};
use super::stream::{Stream, describe};
use super::{at_rules, declarations};
use crate::ast::{ComponentValue, Condition, Rule, Stylesheet};
use crate::token::TokenKind;
use crate::{Error, Result};

/// Parse the whole stream.
pub(super) fn stylesheet(stream: &mut Stream) -> Result<Stylesheet> {
    let mut sheet = Stylesheet::default();
    let mut conditions = Vec::new();
    rule_list(stream, &mut sheet, &mut conditions, false)?;
    Ok(sheet)
}

/// Parse rules until the end of the stream, or until a `}` when `nested`.
pub(super) fn rule_list(
    stream: &mut Stream,
    sheet: &mut Stylesheet,
    conditions: &mut Vec<Condition>,
    nested: bool,
) -> Result<()> {
    loop {
        stream.skip_whitespace();
        match stream.peek() {
            None => return Ok(()),
            Some(TokenKind::CloseCurly) if nested => return Ok(()),
            Some(TokenKind::CloseCurly) => return Err(stream.error("unexpected \"}\"; remove it")),
            Some(TokenKind::AtKeyword(_)) => at_rules::at_rule(stream, sheet, conditions)?,
            Some(_) => qualified_rule(stream, sheet, conditions)?,
        }
    }
}

/// `selector-list { declarations }`, exploded to one rule per selector.
fn qualified_rule(
    stream: &mut Stream,
    sheet: &mut Stylesheet,
    conditions: &[Condition],
) -> Result<()> {
    let position = stream.position();
    let prelude = values_until(stream, &[TokenKind::OpenCurly, TokenKind::Semicolon])?;
    if prelude.is_empty() {
        return Err(Error::new(position, "expected a selector before \"{\""));
    }
    if stream.peek() != Some(&TokenKind::OpenCurly) {
        return Err(stream.error(format!(
            "expected \"{{\" after selector {:?}, found {}",
            crate::serialize::component_values(&prelude),
            describe(stream.peek())
        )));
    }
    let decls = declarations::block(stream)?;
    let parsed = crate::selector::parse_from_prelude(&prelude, position)?;
    for (selector, parsed) in split_commas(&prelude).into_iter().zip(parsed) {
        let position = selector.first().map_or(position, ComponentValue::position);
        sheet.rules.push(Rule {
            prelude: selector,
            selector: parsed,
            conditions: conditions.to_vec(),
            declarations: decls.clone(),
            position,
        });
    }
    Ok(())
}
