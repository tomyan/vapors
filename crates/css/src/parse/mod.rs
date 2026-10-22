//! The parser: tokens to a [`Stylesheet`] (CSS Syntax Level 3 section 5),
//! with every refusal naming its line and column.

mod at_rules;
pub(crate) mod component;
mod declarations;
mod keyframes;
mod rules;
pub(crate) mod stream;

use crate::Result;
use crate::ast::Stylesheet;
use crate::token::tokenize;

/// Parse a stylesheet from text.
///
/// # Errors
/// The first construct refused, with its line and column: malformed
/// syntax, an unknown property or at-rule, or a sumi terminal extra.
pub fn parse(src: &str) -> Result<Stylesheet> {
    let tokens = tokenize(src)?;
    let mut stream = stream::Stream::new(tokens, crate::token::end_position(src));
    rules::stylesheet(&mut stream)
}
