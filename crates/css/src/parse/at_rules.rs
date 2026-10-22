//! At-rules: `@media`, `@container` and `@supports` nest rules under a
//! condition; `@keyframes` defines an animation; everything else is
//! refused by name.

use super::component::values_until;
use super::stream::Stream;
use super::{keyframes, rules};
use crate::ast::{Condition, Stylesheet};
use crate::token::TokenKind;
use crate::{Error, Result};

/// Parse one at-rule; the stream sits on the at-keyword.
pub(super) fn at_rule(
    stream: &mut Stream,
    sheet: &mut Stylesheet,
    conditions: &mut Vec<Condition>,
) -> Result<()> {
    let position = stream.position();
    let Some(TokenKind::AtKeyword(name)) = stream.next().map(|t| t.kind) else {
        unreachable!("caller checked for an at-keyword");
    };
    match name.to_ascii_lowercase().as_str() {
        "media" => conditional(stream, sheet, conditions, "@media", Condition::Media),
        "container" => conditional(
            stream,
            sheet,
            conditions,
            "@container",
            Condition::Container,
        ),
        "supports" => conditional(stream, sheet, conditions, "@supports", Condition::Supports),
        "keyframes" => keyframes::block(stream, sheet, position),
        "layer" => Err(Error::new(
            position,
            "@layer is not supported (sumi has no layers either); order rules by source instead",
        )),
        other => Err(Error::new(
            position,
            format!(
                "unknown at-rule @{other}; supported: @media, @container, @supports, @keyframes"
            ),
        )),
    }
}

/// `@name <prelude> { rules }`: rules inside carry the condition.
fn conditional(
    stream: &mut Stream,
    sheet: &mut Stylesheet,
    conditions: &mut Vec<Condition>,
    name: &str,
    wrap: fn(Vec<crate::ast::ComponentValue>) -> Condition,
) -> Result<()> {
    let position = stream.position();
    let prelude = values_until(stream, &[TokenKind::OpenCurly, TokenKind::Semicolon])?;
    if prelude.is_empty() {
        return Err(Error::new(
            position,
            format!("{name} needs a condition before \"{{\""),
        ));
    }
    let open_at = stream.position();
    stream.expect(
        &TokenKind::OpenCurly,
        &format!("\"{{\" after the {name} condition"),
    )?;
    conditions.push(wrap(prelude));
    rules::rule_list(stream, sheet, conditions, true)?;
    conditions.pop();
    if !stream.eat(&TokenKind::CloseCurly) {
        return Err(Error::new(
            open_at,
            format!("unterminated {name} block; close it with }}"),
        ));
    }
    Ok(())
}
