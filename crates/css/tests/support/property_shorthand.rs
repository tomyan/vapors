//! Helpers that were in `tests/property_shorthand.rs`.
#![allow(dead_code)]

use super::sheet;

pub fn longhands(decl: &str) -> Vec<String> {
    let s = sheet(&format!("a {{ {decl}; }}"));
    s.rules[0].declarations[0]
        .longhands
        .iter()
        .map(|l| format!("{}: {}", l.id.name(), l.value.to_css()))
        .collect()
}
