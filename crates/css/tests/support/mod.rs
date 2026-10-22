//! Shared helpers for the integration tests.
#![allow(dead_code)]

use vapors_css::ast::{Rule, Stylesheet};
use vapors_css::parse;

/// Parse or panic with the refusal text.
pub fn sheet(src: &str) -> Stylesheet {
    match parse(src) {
        Ok(s) => s,
        Err(e) => panic!("parse refused: {e}\n{src}"),
    }
}

/// The refusal for a stylesheet that must not parse.
pub fn refusal(src: &str) -> vapors_css::Error {
    match parse(src) {
        Ok(_) => panic!("expected a refusal for:\n{src}"),
        Err(e) => e,
    }
}

/// The serialized value of a declaration by name, or "" when absent.
pub fn prop(rule: &Rule, name: &str) -> String {
    rule.declarations
        .iter()
        .find(|d| d.name == name)
        .map(vapors_css::ast::Declaration::value_text)
        .unwrap_or_default()
}
