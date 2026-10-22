//! Value syntax: a small grammar language (the shape of CSS value
//! definitions, see Values Level 4 section 2) parsed once per property,
//! and a matcher that turns component values into a [`Value`].
//!
//! Supported: keywords, `<type>` with an optional `[lo,hi]` range,
//! `|` alternatives, juxtaposition, `[...]` groups, and the multipliers
//! `?`, `+`, `*`, `#` and `{a,b}`. `||` and `&&` are not supported;
//! shorthands that need them are expanded by hand in `property::shorthand`.

mod grammar;
mod matcher;
mod special;
mod special_grid;
mod types;

pub use grammar::{Grammar, parse_grammar};

use crate::ast::ComponentValue;
use crate::value::Value;

/// Match `values` (whitespace included) against `grammar`. `None` when
/// the whole input does not match.
#[must_use]
pub fn match_value(grammar: &Grammar, values: &[ComponentValue]) -> Option<Value> {
    let items: Vec<&ComponentValue> = values.iter().filter(|v| !v.is_whitespace()).collect();
    matcher::match_all(grammar, &items)
}
